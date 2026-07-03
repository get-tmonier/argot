//! Call-receiver scorer — port of
//! `engine/argot/scoring/scorers/call_receiver.py`.
//!
//! Tracks distinct call-expression callees across the repo corpus and
//! penalises unattested callees in a hunk (soft additive BPE penalty).
//! Cluster-conditional attestation groups files by callee-bag similarity
//! (MinHash + KMeans) and adds `cluster_bonus` for globally-attested callees
//! absent from the hunk-file's cluster.
//!
//! KMeans note: sklearn's exact seed-0 partition is not reproducible
//! cross-implementation (see docs/rust-port/PORTING-NOTES.md). We use a
//! deterministic hand-rolled k-means++; scoring is invariant to cluster-label
//! numbering, and cluster-affected scores are gated on the benchmark AUC
//! rather than byte-parity, per the recorded decision.

use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::minhash_params_seed0::{MINHASH_A_SEED0, MINHASH_B_SEED0};
use crate::scoring::model::{CallReceiverModel, ClusterModel};
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::ts_parse::parse;
use md5::{Digest, Md5};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

const MINHASH_PRIME: u64 = (1 << 31) - 1;
const MINHASH_N_PERMS: usize = 128;

fn node_text(node: Node, src: &[u8]) -> String {
    let r = node.byte_range();
    if r.is_empty() {
        String::new()
    } else {
        String::from_utf8_lossy(&src[r]).into_owned()
    }
}

fn py_call_types(kind: &str) -> bool {
    kind == "call"
}
fn ts_call_types(kind: &str) -> bool {
    kind == "call_expression" || kind == "new_expression"
}
fn go_call_types(kind: &str) -> bool {
    kind == "call_expression"
}
fn rust_call_types(kind: &str) -> bool {
    // Macros are a first-class part of Rust's call surface (`println!`, `vec!`),
    // so they count alongside plain call expressions.
    kind == "call_expression" || kind == "macro_invocation"
}
fn c_call_types(kind: &str) -> bool {
    crate::scoring::adapters::c::is_call_kind(kind)
}

fn extract_c_callee(call_node: Node, src: &[u8]) -> Option<String> {
    crate::scoring::adapters::c::callee(call_node, src)
}
fn java_call_types(kind: &str) -> bool {
    kind == "method_invocation" || kind == "object_creation_expression"
}
fn cs_call_types(kind: &str) -> bool {
    kind == "invocation_expression" || kind == "object_creation_expression"
}
fn cpp_call_types(kind: &str) -> bool {
    kind == "call_expression"
}
fn rb_call_types(kind: &str) -> bool {
    kind == "call"
}

fn extract_python_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "attribute" {
        let attr = callee.child_by_field_name("attribute")?;
        let obj = callee.child_by_field_name("object")?;
        parts.insert(0, node_text(attr, src));
        callee = obj;
    }
    if callee.kind() == "identifier" {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if py_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else {
        None
    }
}

/// Dotted-callee signature for a Ruby `call` node, keyed by its receiver.
///
/// Ruby nests a dotted chain (`a.b.c`) through the `receiver` field rather than
/// via separate attribute nodes, so we walk down receivers collecting method
/// segments. A bare call (no receiver) yields just its method name (self
/// scope). A literal/complex receiver is dropped (`None`), matching the Python
/// extractor's "only identifier/constant heads survive" rule.
fn extract_ruby_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut node = call_node;
    loop {
        match node.child_by_field_name("method") {
            Some(m) => parts.insert(0, node_text(m, src)),
            None => parts.insert(0, "<call>".to_string()),
        }
        match node.child_by_field_name("receiver") {
            None => return Some(parts.join(".")),
            Some(recv) => match recv.kind() {
                "call" => {
                    node = recv;
                }
                "identifier" | "constant" | "instance_variable" | "class_variable"
                | "global_variable" | "self" | "scope_resolution" => {
                    parts.insert(0, node_text(recv, src));
                    return Some(parts.join("."));
                }
                _ => return None,
            },
        }
    }
}

fn extract_typescript_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let field = if call_node.kind() == "new_expression" {
        "constructor"
    } else {
        "function"
    };
    let mut callee = call_node.child_by_field_name(field)?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "member_expression" {
        let prop = callee.child_by_field_name("property")?;
        let obj = callee.child_by_field_name("object")?;
        parts.insert(0, node_text(prop, src));
        callee = obj;
    }
    if callee.kind() == "identifier" || callee.kind() == "type_identifier" {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if ts_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else if matches!(callee.kind(), "this" | "super") {
        // `this.method()` / `super.method()` — Python's `self.method` has
        // always been extracted (self is a plain identifier); TypeScript's
        // `this` is its own node kind, which the original walk dropped,
        // leaving class-internal call voice invisible (the legacy-lifecycle
        // and class-component break families all live in this namespace).
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else {
        None
    }
}

fn extract_go_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "selector_expression" {
        let field = callee.child_by_field_name("field")?;
        let operand = callee.child_by_field_name("operand")?;
        parts.insert(0, node_text(field, src));
        callee = operand;
    }
    if matches!(
        callee.kind(),
        "identifier" | "field_identifier" | "type_identifier" | "package_identifier"
    ) {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if go_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else {
        None
    }
}

fn extract_rust_callee(call_node: Node, src: &[u8]) -> Option<String> {
    // `foo!(…)` — the macro name, tagged with `!` to keep macro and function
    // namespaces distinct.
    if call_node.kind() == "macro_invocation" {
        let mac = call_node.child_by_field_name("macro")?;
        return Some(format!("{}!", node_text(mac, src)));
    }
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    // Unwind method-call chains `recv.a().b()` into dotted parts.
    while callee.kind() == "field_expression" {
        let field = callee.child_by_field_name("field")?;
        let value = callee.child_by_field_name("value")?;
        parts.insert(0, node_text(field, src));
        callee = value;
    }
    if matches!(
        callee.kind(),
        "identifier" | "field_identifier" | "type_identifier" | "scoped_identifier"
    ) {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if rust_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else {
        None
    }
}

/// Simple type name of a constructor's `type` node
/// (`new java.util.ArrayList<String>()` → `ArrayList`).
fn java_type_simple_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" => Some(node_text(node, src)),
        "generic_type" => {
            let mut cursor = node.walk();
            let named: Option<Node> = node.children(&mut cursor).find(|c| c.is_named());
            named.and_then(|c| java_type_simple_name(c, src))
        }
        "scoped_type_identifier" => node
            .child_by_field_name("name")
            .map(|n| node_text(n, src))
            .or_else(|| Some(node_text(node, src))),
        _ => Some(node_text(node, src)),
    }
}

/// Build the dotted receiver chain for a `method_invocation` object node.
/// Mirrors [`extract_typescript_callee`]'s member walk: a call in the chain
/// contributes the `<call>` sentinel; an unmodelled base returns `None`.
fn build_java_receiver(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(node_text(node, src)),
        "this" => Some("this".to_string()),
        "super" => Some("super".to_string()),
        "method_invocation" | "object_creation_expression" => Some("<call>".to_string()),
        "field_access" => {
            let field = node.child_by_field_name("field")?;
            let field_text = node_text(field, src);
            match node.child_by_field_name("object") {
                Some(obj) => {
                    let base =
                        build_java_receiver(obj, src).unwrap_or_else(|| "<call>".to_string());
                    Some(format!("{base}.{field_text}"))
                }
                None => Some(field_text),
            }
        }
        _ => None,
    }
}

fn cs_named_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let children: Vec<Node<'a>> = node.children(&mut cursor).collect();
    children.into_iter().find(|c| c.kind() == kind)
}

/// C# type name for `object_creation_expression` (generics stripped), or the
/// dotted callee for `invocation_expression`. Mirrors `csharp.rs`.
fn extract_csharp_callee(call_node: Node, src: &[u8]) -> Option<String> {
    if call_node.kind() == "object_creation_expression" {
        let ty = call_node.child_by_field_name("type")?;
        return match ty.kind() {
            "identifier" | "qualified_name" => Some(node_text(ty, src)),
            "generic_name" => cs_named_child_of_kind(ty, "identifier").map(|id| node_text(id, src)),
            _ => None,
        };
    }
    // invocation_expression
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "member_access_expression" {
        let name = callee.child_by_field_name("name")?;
        parts.insert(0, node_text(name, src));
        match callee.child_by_field_name("expression") {
            Some(expr) => callee = expr,
            None => return Some(parts.join(".")),
        }
    }
    match callee.kind() {
        "identifier" => {
            parts.insert(0, node_text(callee, src));
            Some(parts.join("."))
        }
        "generic_name" => {
            let id = cs_named_child_of_kind(callee, "identifier")?;
            parts.insert(0, node_text(id, src));
            Some(parts.join("."))
        }
        // `this.M()` / `base.M()` — anonymous receiver nodes kept for
        // class-internal call voice (mirrors the TS `this`/`super` handling).
        "this" | "base" => {
            parts.insert(0, node_text(callee, src));
            Some(parts.join("."))
        }
        k if cs_call_types(k) => {
            parts.insert(0, "<call>".to_string());
            Some(parts.join("."))
        }
        _ => None,
    }
}

fn php_call_types(kind: &str) -> bool {
    matches!(
        kind,
        "function_call_expression"
            | "member_call_expression"
            | "nullsafe_member_call_expression"
            | "scoped_call_expression"
            | "object_creation_expression"
    )
}

/// Resolve a PHP expression node to a dotted receiver path (e.g. `$this.foo`,
/// `Response`), or `None` when the chain bottoms out at something dynamic.
fn php_expr_dotted(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "name" | "qualified_name" | "variable_name" => Some(node_text(node, src)),
        "member_access_expression" | "nullsafe_member_access_expression" => {
            let obj = node.child_by_field_name("object")?;
            let name = node.child_by_field_name("name")?;
            let base = php_expr_dotted(obj, src)?;
            Some(format!("{base}.{}", node_text(name, src)))
        }
        "scoped_property_access_expression" => {
            let scope = node.child_by_field_name("scope")?;
            let name = node.child_by_field_name("name")?;
            let base = php_expr_dotted(scope, src)?;
            Some(format!("{base}.{}", node_text(name, src)))
        }
        k if php_call_types(k) => Some("<call>".to_string()),
        _ => None,
    }
}

fn extract_php_callee(call_node: Node, src: &[u8]) -> Option<String> {
    match call_node.kind() {
        // `new Type(...)` — the receiver is the constructed type name.
        "object_creation_expression" => {
            let mut cursor = call_node.walk();
            for child in call_node.children(&mut cursor) {
                if matches!(child.kind(), "name" | "qualified_name" | "variable_name") {
                    return Some(node_text(child, src));
                }
            }
            None
        }
        "function_call_expression" => {
            let f = call_node.child_by_field_name("function")?;
            php_expr_dotted(f, src)
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            let obj = call_node.child_by_field_name("object")?;
            let name = call_node.child_by_field_name("name")?;
            let base = php_expr_dotted(obj, src)?;
            Some(format!("{base}.{}", node_text(name, src)))
        }
        "scoped_call_expression" => {
            let scope = call_node.child_by_field_name("scope")?;
            let name = call_node.child_by_field_name("name")?;
            let base = php_expr_dotted(scope, src)?;
            Some(format!("{base}.{}", node_text(name, src)))
        }
        _ => None,
    }
}

fn extract_cpp_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "field_expression" {
        let field = callee.child_by_field_name("field")?;
        let obj = callee.child_by_field_name("argument")?;
        parts.insert(0, node_text(field, src));
        callee = obj;
    }
    match callee.kind() {
        "identifier" | "field_identifier" => {
            parts.insert(0, node_text(callee, src));
            Some(parts.join("."))
        }
        // `Foo::bar` → dotted `Foo.bar`; the leading segment is the receiver
        // namespace, mirroring how `self.method` / `obj.method` are keyed.
        "qualified_identifier" => {
            parts.insert(0, node_text(callee, src).replace("::", "."));
            Some(parts.join("."))
        }
        "call_expression" => {
            parts.insert(0, "<call>".to_string());
            Some(parts.join("."))
        }
        _ => None,
    }
}

fn extract_java_callee(call_node: Node, src: &[u8]) -> Option<String> {
    if call_node.kind() == "object_creation_expression" {
        let ty = call_node.child_by_field_name("type")?;
        return java_type_simple_name(ty, src);
    }
    let name = call_node.child_by_field_name("name")?;
    let method = node_text(name, src);
    match call_node.child_by_field_name("object") {
        None => Some(method),
        Some(obj) => {
            let base = build_java_receiver(obj, src)?;
            Some(format!("{base}.{method}"))
        }
    }
}

fn walk_preorder(root: Node, mut visit: impl FnMut(Node)) {
    // Stack DFS pushing reversed children, matching Python `_walk_nodes`.
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

/// Whether any direct child of the parse-tree root is an ERROR node — a
/// fragment we should not extract callees from. Parse failure → true.
pub fn has_root_error(source: &str, language: Language) -> bool {
    match parse(source, language) {
        None => true,
        Some(tree) => {
            let root = tree.root_node();
            let mut cursor = root.walk();
            let has_error = root.children(&mut cursor).any(|c| c.kind() == "ERROR");
            has_error
        }
    }
}

/// Return dotted-callee signatures for every call-expression in `source`
/// (`None` entries preserved for auditing). `[]` on parse error / empty.
pub fn extract_callees(source: &str, language: Language) -> Vec<Option<String>> {
    if source.trim().is_empty() {
        return Vec::new();
    }
    let tree = match parse(source, language) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let bytes = source.as_bytes();
    let mut out: Vec<Option<String>> = Vec::new();
    let is_call = match language {
        Language::Python => py_call_types as fn(&str) -> bool,
        Language::Typescript => ts_call_types as fn(&str) -> bool,
        Language::Go => go_call_types as fn(&str) -> bool,
        Language::Rust => rust_call_types as fn(&str) -> bool,
        Language::C => c_call_types as fn(&str) -> bool,
        Language::Java => java_call_types as fn(&str) -> bool,
        Language::CSharp => cs_call_types as fn(&str) -> bool,
        Language::Php => php_call_types as fn(&str) -> bool,
        Language::Cpp => cpp_call_types as fn(&str) -> bool,
        Language::Ruby => rb_call_types as fn(&str) -> bool,
    };
    let extractor = match language {
        Language::Python => extract_python_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Typescript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Go => extract_go_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Rust => extract_rust_callee as fn(Node, &[u8]) -> Option<String>,
        Language::C => extract_c_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Java => extract_java_callee as fn(Node, &[u8]) -> Option<String>,
        Language::CSharp => extract_csharp_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Php => extract_php_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Cpp => extract_cpp_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Ruby => extract_ruby_callee as fn(Node, &[u8]) -> Option<String>,
    };
    walk_preorder(tree.root_node(), |node| {
        if is_call(node.kind()) {
            out.push(extractor(node, bytes));
        }
    });
    out
}

fn non_none_callees(source: &str, language: Language) -> Vec<String> {
    extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect()
}

/// The leading module/namespace segment of a callee — the first non-empty
/// token when split on `.`, `::`, or `\`. A bare name (no separator) returns
/// the whole string. `\Doctrine\ORM\X.create` → `Doctrine`,
/// `tokio::runtime::Runtime::new` → `tokio`, `viper.GetString` → `viper`,
/// `String::with_capacity` → `String`. Used to decide whether an unattested
/// callee reaches into a module foreign to the repo (`tokio`) or a known /
/// local one (`String`, `self`, a local receiver variable).
fn leading_namespace(callee: &str) -> &str {
    let trimmed = callee.trim_start_matches('\\');
    let end = trimmed.find(['.', ':', '\\']).unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// Whether `callee` carries a receiver/namespace qualifier (a `.`, `::`, or
/// `\`). A bare identifier is unqualified.
fn is_qualified(callee: &str) -> bool {
    callee.contains('.') || callee.contains(':') || callee.contains('\\')
}

/// Callees of every call-expression whose start line falls inside the
/// 1-indexed inclusive `[start_line, end_line]` region of `source`.
///
/// Parse-error host fallback: when a bare hunk's parse has root-level errors,
/// callee extraction falls back to the hunk's region within its host file's
/// AST — the host parses cleanly where the fragment did not.
pub fn callees_in_source_region(
    source: &str,
    language: Language,
    start_line: usize,
    end_line: usize,
) -> Vec<String> {
    let tree = match parse(source, language) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let bytes = source.as_bytes();
    let is_call = match language {
        Language::Python => py_call_types as fn(&str) -> bool,
        Language::Typescript => ts_call_types as fn(&str) -> bool,
        Language::Go => go_call_types as fn(&str) -> bool,
        Language::Rust => rust_call_types as fn(&str) -> bool,
        Language::C => c_call_types as fn(&str) -> bool,
        Language::Java => java_call_types as fn(&str) -> bool,
        Language::CSharp => cs_call_types as fn(&str) -> bool,
        Language::Php => php_call_types as fn(&str) -> bool,
        Language::Cpp => cpp_call_types as fn(&str) -> bool,
        Language::Ruby => rb_call_types as fn(&str) -> bool,
    };
    let extractor = match language {
        Language::Python => extract_python_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Typescript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Go => extract_go_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Rust => extract_rust_callee as fn(Node, &[u8]) -> Option<String>,
        Language::C => extract_c_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Java => extract_java_callee as fn(Node, &[u8]) -> Option<String>,
        Language::CSharp => extract_csharp_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Php => extract_php_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Cpp => extract_cpp_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Ruby => extract_ruby_callee as fn(Node, &[u8]) -> Option<String>,
    };
    let mut out = Vec::new();
    walk_preorder(tree.root_node(), |node| {
        if is_call(node.kind()) {
            let line = node.start_position().row + 1;
            if line >= start_line && line <= end_line {
                if let Some(c) = extractor(node, bytes) {
                    out.push(c);
                }
            }
        }
    });
    out
}

/// 128-element MinHash signature over a callee bag using the seed-0 universal
/// hash family. Empty bag → all zeros.
pub fn minhash_signature(bag: &HashSet<String>) -> Vec<i64> {
    if bag.is_empty() {
        return vec![0; MINHASH_N_PERMS];
    }
    let mut sig = vec![MINHASH_PRIME; MINHASH_N_PERMS];
    for callee in bag {
        let mut hasher = Md5::new();
        hasher.update(callee.as_bytes());
        let digest = hasher.finalize();
        let mut first8 = [0u8; 8];
        first8.copy_from_slice(&digest[..8]);
        let h = u64::from_le_bytes(first8) % MINHASH_PRIME;
        for i in 0..MINHASH_N_PERMS {
            let v = (MINHASH_A_SEED0[i]
                .wrapping_mul(h)
                .wrapping_add(MINHASH_B_SEED0[i]))
                % MINHASH_PRIME;
            if v < sig[i] {
                sig[i] = v;
            }
        }
    }
    sig.into_iter().map(|v| v as i64).collect()
}

// ---------------------------------------------------------------------------
// Deterministic hand-rolled k-means++ (AUC-fallback for sklearn KMeans).
// ---------------------------------------------------------------------------

struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn next_f64(&mut self) -> f64 {
        // 53-bit mantissa uniform in [0,1).
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn dist_sq(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn kmeans_plus_plus(data: &[Vec<f64>], k: usize, rng: &mut SplitMix64) -> Vec<Vec<f64>> {
    let n = data.len();
    let mut centers: Vec<Vec<f64>> = Vec::with_capacity(k);
    let first = (rng.next_f64() * n as f64) as usize % n;
    centers.push(data[first].clone());
    let mut closest: Vec<f64> = data.iter().map(|p| dist_sq(p, &centers[0])).collect();
    while centers.len() < k {
        let total: f64 = closest.iter().sum();
        if total <= 0.0 {
            // All remaining points coincide with a center; pad with an
            // arbitrary point deterministically.
            centers.push(data[centers.len() % n].clone());
            continue;
        }
        let target = rng.next_f64() * total;
        let mut acc = 0.0;
        let mut chosen = n - 1;
        for (i, d) in closest.iter().enumerate() {
            acc += d;
            if acc >= target {
                chosen = i;
                break;
            }
        }
        centers.push(data[chosen].clone());
        let c = centers.last().unwrap();
        for (i, p) in data.iter().enumerate() {
            let d = dist_sq(p, c);
            if d < closest[i] {
                closest[i] = d;
            }
        }
    }
    centers
}

fn lloyd(data: &[Vec<f64>], mut centers: Vec<Vec<f64>>, max_iter: usize) -> (Vec<usize>, f64) {
    let k = centers.len();
    let dim = data[0].len();
    let mut labels = vec![0usize; data.len()];
    for _ in 0..max_iter {
        let mut changed = false;
        for (i, p) in data.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f64::INFINITY;
            for (c, center) in centers.iter().enumerate() {
                let d = dist_sq(p, center);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
            if labels[i] != best {
                labels[i] = best;
                changed = true;
            }
        }
        // Recompute centers.
        let mut sums = vec![vec![0.0f64; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, p) in data.iter().enumerate() {
            let c = labels[i];
            counts[c] += 1;
            for d in 0..dim {
                sums[c][d] += p[d];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..dim {
                    centers[c][d] = sums[c][d] / counts[c] as f64;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let inertia: f64 = data
        .iter()
        .enumerate()
        .map(|(i, p)| dist_sq(p, &centers[labels[i]]))
        .sum();
    (labels, inertia)
}

fn kmeans(data: &[Vec<f64>], k: usize, seed: u64, n_init: usize) -> Vec<usize> {
    let mut best_labels: Option<Vec<usize>> = None;
    let mut best_inertia = f64::INFINITY;
    for init in 0..n_init {
        let mut rng = SplitMix64::new(seed.wrapping_add(init as u64).wrapping_add(1));
        let centers = kmeans_plus_plus(data, k, &mut rng);
        let (labels, inertia) = lloyd(data, centers, 300);
        if inertia < best_inertia {
            best_inertia = inertia;
            best_labels = Some(labels);
        }
    }
    best_labels.unwrap_or_else(|| vec![0; data.len()])
}

/// Cluster files by normalized MinHash signatures. Returns
/// (file→cluster, cluster→size). Deterministic.
fn cluster_by_signatures(
    file_sigs: &[(PathBuf, Vec<i64>)],
    n_clusters: usize,
    seed: u64,
) -> (HashMap<PathBuf, usize>, HashMap<usize, usize>) {
    let n = file_sigs.len();
    let effective_k = n_clusters.min(n);
    let data: Vec<Vec<f64>> = file_sigs
        .iter()
        .map(|(_, s)| s.iter().map(|&v| v as f64 / MINHASH_PRIME as f64).collect())
        .collect();
    let labels: Vec<usize> = if effective_k <= 1 {
        vec![0; n]
    } else {
        kmeans(&data, effective_k, seed, 10)
    };
    let mut file_to_cluster = HashMap::new();
    for (i, (p, _)) in file_sigs.iter().enumerate() {
        file_to_cluster.insert(p.clone(), labels[i]);
    }
    let mut cluster_sizes = HashMap::new();
    for cid in 0..effective_k {
        cluster_sizes.insert(cid, labels.iter().filter(|&&l| l == cid).count());
    }
    (file_to_cluster, cluster_sizes)
}

/// Names the change binds locally, split by how they attest callees.
///
/// * `callables` — function/class definitions, function-valued variables,
///   import bindings, changeset-wide definitions. Attest bare callees AND
///   dotted callees via their root (`helpers.run()` with `helpers` imported).
/// * `values` — every other local value binding (destructured names, plain
///   consts/vars, parameters). Attest BARE callees only: calling a value you
///   just bound (a `useState` setter, an injected callback) is neighbourhood
///   behaviour, but a dotted method on it (`xhr.open`) still carries voice.
#[derive(Debug, Clone, Default)]
pub struct LocalBindings {
    pub callables: HashSet<String>,
    pub values: HashSet<String>,
}

impl LocalBindings {
    /// Whether `callee` is attested by these bindings.
    fn attests(&self, callee: &str) -> bool {
        let root = callee.split_once('.').map(|(h, _)| h).unwrap_or(callee);
        if self.callables.contains(callee) || self.callables.contains(root) {
            return true;
        }
        !callee.contains('.') && self.values.contains(callee)
    }
}

/// Which rule a distinct hunk callee triggered in
/// [`CallReceiverScorer::weighted_contribution_for_file`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionBranch {
    /// Globally unattested, attested root → `alpha + root_bonus`.
    UnattestedKnownRoot,
    /// Globally unattested, unknown root → `alpha`.
    Unattested,
    /// Globally attested but absent from the file's cluster → `cluster_bonus`.
    ClusterAbsent,
    /// Attested in ≤ rare-threshold cluster files → `cluster_bonus`.
    ClusterRare,
}

/// One distinct callee's contribution decision (scout/evidence surface).
#[derive(Debug, Clone)]
pub struct ContributionEvent {
    pub callee: String,
    pub branch: ContributionBranch,
}

/// Rarity weighting for the cluster branches (`ClusterAbsent`,
/// `ClusterRare`): scales `cluster_bonus` by how globally common the callee is
/// in the corpus, so locally-rare-but-globally-rare callees (locale
/// identifiers, Zipf-tail helpers) stop firing full-magnitude bonuses while
/// globally-common callees absent from a cluster keep them. All weights are
/// derived from corpus document frequencies — no domain knowledge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RarityWeighting {
    /// Baseline behaviour with the rule off: full `cluster_bonus` regardless
    /// of global rarity.
    Off,
    /// `weight = df / N` — proportional to document frequency.
    LinearDf,
    /// `weight = 1 if df ≥ min_df else 0` — hard gate on document frequency.
    GatedDf { min_df: usize },
    /// `weight = ln(1 + df) / ln(1 + N)` — logarithmic soft gate.
    LogDf,
}

impl RarityWeighting {
    /// Weight in [0, 1] for a callee seen in `df` of `n_files` corpus files.
    pub fn weight(&self, df: usize, n_files: usize) -> f64 {
        match self {
            RarityWeighting::Off => 1.0,
            RarityWeighting::LinearDf => {
                if n_files == 0 {
                    1.0
                } else {
                    df as f64 / n_files as f64
                }
            }
            RarityWeighting::GatedDf { min_df } => {
                if df >= *min_df {
                    1.0
                } else {
                    0.0
                }
            }
            RarityWeighting::LogDf => {
                if n_files == 0 {
                    1.0
                } else {
                    (1.0 + df as f64).ln() / (1.0 + n_files as f64).ln()
                }
            }
        }
    }
}

/// Call-receiver scorer.
pub struct CallReceiverScorer {
    language: Language,
    pub alpha: f64,
    pub cap: usize,
    cluster_rare_threshold: usize,
    cluster_size_min: usize,
    attested: HashSet<String>,
    attested_roots: HashSet<String>,
    /// Last segments of dotted attested callees — the corpus's known method
    /// vocabulary. A dotted callee is keyed by its receiver's variable name,
    /// so a fresh receiver makes `widgetTitles.join` look unattested even
    /// though the corpus calls `.join` constantly; the method segment is
    /// what carries voice, so corpus-known methods do not alpha-fire.
    attested_methods: HashSet<String>,
    /// Leading module/namespace segments of every attested callee — the set
    /// of modules the repo already reaches into (`String`, `self`, `gix`,
    /// every local receiver name). An unattested callee qualified by a
    /// namespace *outside* this set (`tokio`, `viper`, `\Doctrine`) reaches
    /// into a foreign module; one inside it (`String::with_capacity`,
    /// `repository.set_resource`) is a new API on a known/local receiver.
    attested_namespaces: HashSet<String>,
    pub n_skipped_data_dominant: usize,
    file_to_cluster: HashMap<PathBuf, usize>,
    cluster_attested: HashMap<usize, HashSet<String>>,
    cluster_callee_counts: HashMap<usize, HashMap<String, usize>>,
    cluster_sizes: HashMap<usize, usize>,
    /// Corpus-global document frequency: number of corpus files whose callee
    /// bag contains each callee (rarity weighting substrate).
    callee_file_counts: HashMap<String, usize>,
    /// Number of (non-data-dominant) corpus files behind `callee_file_counts`.
    n_corpus_files: usize,
    /// Rarity weighting applied to the cluster branches.
    rarity_weighting: RarityWeighting,
    /// Additive per-cluster AST-shape primitives (empty = true no-op).
    shape_primitives: Vec<Box<dyn ShapePrimitive>>,
    /// primitive name → cluster id → fitted baseline.
    primitive_baselines: HashMap<String, HashMap<usize, Baseline>>,
    /// Per-primitive fire counts (bench observability).
    pub primitive_fire_count: HashMap<String, usize>,
    pub rare_branch_fire_count: usize,
    pub rare_branch_hunks_fired: usize,
    pub hunks_scored: usize,
}

impl CallReceiverScorer {
    /// Fit over `repo_files` (path + already-read source). `adapter` is used
    /// only for `is_data_dominant`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo_files: &[(PathBuf, String)],
        language: Language,
        alpha: f64,
        cap: usize,
        adapter: &dyn LanguageAdapter,
        n_clusters: usize,
        cluster_seed: u64,
        cluster_rare_threshold: usize,
        cluster_size_min: usize,
    ) -> Result<Self, &'static str> {
        let mut attested: HashSet<String> = HashSet::new();
        let mut skipped = 0usize;
        let mut files_list: Vec<PathBuf> = Vec::new();
        let mut file_bags: Vec<(PathBuf, HashSet<String>)> = Vec::new();
        let mut file_sigs: Vec<(PathBuf, Vec<i64>)> = Vec::new();

        let mut callee_file_counts: HashMap<String, usize> = HashMap::new();
        for (path, src) in repo_files {
            if adapter.is_data_dominant(src) {
                skipped += 1;
                continue;
            }
            let callees = non_none_callees(src, language);
            for c in &callees {
                attested.insert(c.clone());
            }
            files_list.push(path.clone());
            let bag: HashSet<String> = callees.into_iter().collect();
            for callee in &bag {
                *callee_file_counts.entry(callee.clone()).or_insert(0) += 1;
            }
            if n_clusters > 1 {
                let sig = minhash_signature(&bag);
                file_bags.push((path.clone(), bag));
                file_sigs.push((path.clone(), sig));
            }
        }

        if files_list.is_empty() {
            return Err("repo_corpus_files must be non-empty");
        }

        let attested_roots: HashSet<String> = attested
            .iter()
            .map(|c| c.split_once('.').map(|(h, _)| h).unwrap_or(c).to_string())
            .collect();
        let attested_methods: HashSet<String> = attested
            .iter()
            .filter_map(|c| c.rsplit_once('.').map(|(_, m)| m.to_string()))
            .collect();
        let attested_namespaces: HashSet<String> = attested
            .iter()
            .map(|c| leading_namespace(c).to_string())
            .collect();

        let mut file_to_cluster = HashMap::new();
        let mut cluster_attested: HashMap<usize, HashSet<String>> = HashMap::new();
        let mut cluster_callee_counts: HashMap<usize, HashMap<String, usize>> = HashMap::new();
        let mut cluster_sizes = HashMap::new();

        if n_clusters > 1 && !file_sigs.is_empty() {
            let (ftc, sizes) = cluster_by_signatures(&file_sigs, n_clusters, cluster_seed);
            file_to_cluster = ftc;
            cluster_sizes = sizes;
            let effective_k = cluster_sizes.len();
            for cid in 0..effective_k {
                cluster_callee_counts.insert(cid, HashMap::new());
            }
            // Per-file presence counts.
            for (path, bag) in &file_bags {
                if let Some(&cid) = file_to_cluster.get(path) {
                    let counts = cluster_callee_counts.get_mut(&cid).unwrap();
                    for callee in bag {
                        *counts.entry(callee.clone()).or_insert(0) += 1;
                    }
                }
            }
            for (cid, counts) in &cluster_callee_counts {
                cluster_attested.insert(*cid, counts.keys().cloned().collect());
            }
        }

        let n_corpus_files = files_list.len();
        Ok(Self {
            language,
            alpha,
            cap,
            cluster_rare_threshold,
            cluster_size_min,
            attested,
            attested_roots,
            attested_methods,
            n_skipped_data_dominant: skipped,
            file_to_cluster,
            cluster_attested,
            cluster_callee_counts,
            cluster_sizes,
            callee_file_counts,
            n_corpus_files,
            attested_namespaces,
            rarity_weighting: RarityWeighting::Off,
            shape_primitives: Vec::new(),
            primitive_baselines: HashMap::new(),
            primitive_fire_count: HashMap::new(),
            rare_branch_fire_count: 0,
            rare_branch_hunks_fired: 0,
            hunks_scored: 0,
        })
    }

    /// Export the fitted state as a [`CallReceiverModel`] snapshot. File keys
    /// are relativized against `repo_root` (paths outside the root keep their
    /// full string form) so the artifact is repo-relative and matches the
    /// paths `check` sees from git.
    pub fn export_model(&self, repo_root: &Path) -> CallReceiverModel {
        let rel = |p: &Path| {
            crate::suppress::rel_string(p, repo_root)
                .unwrap_or_else(|| p.to_string_lossy().into_owned())
        };
        let mut clusters: BTreeMap<String, ClusterModel> = BTreeMap::new();
        let mut cids: Vec<usize> = self.cluster_sizes.keys().copied().collect();
        cids.sort_unstable();
        for cid in cids {
            let mut files: Vec<String> = self
                .file_to_cluster
                .iter()
                .filter(|(_, &c)| c == cid)
                .map(|(p, _)| rel(p))
                .collect();
            files.sort();
            let callee_counts: BTreeMap<String, usize> = self
                .cluster_callee_counts
                .get(&cid)
                .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect())
                .unwrap_or_default();
            clusters.insert(
                cid.to_string(),
                ClusterModel {
                    files,
                    callee_counts,
                },
            );
        }
        let mut attested: Vec<String> = self.attested.iter().cloned().collect();
        attested.sort();
        CallReceiverModel {
            attested,
            n_corpus_files: self.n_corpus_files,
            clusters,
        }
    }

    /// Rebuild a scorer from a fit-time [`CallReceiverModel`] snapshot — the
    /// check-time path. The snapshot pins attestation and cluster membership
    /// to fit time, so new code cannot attest its own callees (issue #79).
    /// Document frequencies are recovered as the per-callee sum across
    /// clusters (each corpus file belongs to exactly one cluster).
    pub fn from_model(
        model: &CallReceiverModel,
        language: Language,
        alpha: f64,
        cap: usize,
        cluster_rare_threshold: usize,
        cluster_size_min: usize,
    ) -> Result<Self, &'static str> {
        let attested: HashSet<String> = model.attested.iter().cloned().collect();
        let attested_roots: HashSet<String> = attested
            .iter()
            .map(|c| c.split_once('.').map(|(h, _)| h).unwrap_or(c).to_string())
            .collect();
        let attested_methods: HashSet<String> = attested
            .iter()
            .filter_map(|c| c.rsplit_once('.').map(|(_, m)| m.to_string()))
            .collect();
        let attested_namespaces: HashSet<String> = attested
            .iter()
            .map(|c| leading_namespace(c).to_string())
            .collect();
        let mut file_to_cluster = HashMap::new();
        let mut cluster_attested: HashMap<usize, HashSet<String>> = HashMap::new();
        let mut cluster_callee_counts: HashMap<usize, HashMap<String, usize>> = HashMap::new();
        let mut cluster_sizes = HashMap::new();
        let mut callee_file_counts: HashMap<String, usize> = HashMap::new();
        for (cid_str, cluster) in &model.clusters {
            let cid: usize = cid_str.parse().map_err(|_| "invalid cluster id")?;
            for f in &cluster.files {
                file_to_cluster.insert(PathBuf::from(f), cid);
            }
            cluster_sizes.insert(cid, cluster.files.len());
            cluster_attested.insert(cid, cluster.callee_counts.keys().cloned().collect());
            let counts: HashMap<String, usize> = cluster
                .callee_counts
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (callee, n) in &counts {
                *callee_file_counts.entry(callee.clone()).or_insert(0) += n;
            }
            cluster_callee_counts.insert(cid, counts);
        }
        Ok(Self {
            language,
            alpha,
            cap,
            cluster_rare_threshold,
            cluster_size_min,
            attested,
            attested_roots,
            attested_methods,
            n_skipped_data_dominant: 0,
            file_to_cluster,
            cluster_attested,
            cluster_callee_counts,
            cluster_sizes,
            callee_file_counts,
            n_corpus_files: model.n_corpus_files,
            attested_namespaces,
            rarity_weighting: RarityWeighting::Off,
            shape_primitives: Vec::new(),
            primitive_baselines: HashMap::new(),
            primitive_fire_count: HashMap::new(),
            rare_branch_fire_count: 0,
            rare_branch_hunks_fired: 0,
            hunks_scored: 0,
        })
    }

    /// Set the rarity weighting for the cluster branches.
    pub fn with_rarity_weighting(mut self, weighting: RarityWeighting) -> Self {
        self.rarity_weighting = weighting;
        self
    }

    /// Attach additive shape primitives and fit their per-cluster baselines
    /// over the (non-data-dominant, clustered) corpus files. An empty
    /// primitive list is a true no-op, matching the Python scorer.
    pub fn with_shape_primitives(
        mut self,
        primitives: Vec<Box<dyn ShapePrimitive>>,
        repo_files: &[(PathBuf, String)],
        adapter: &dyn LanguageAdapter,
    ) -> Self {
        if primitives.is_empty() {
            return self;
        }
        let mut cluster_files: HashMap<usize, Vec<(PathBuf, String)>> = HashMap::new();
        for (path, src) in repo_files {
            if adapter.is_data_dominant(src) {
                continue;
            }
            if let Some(&cid) = self.file_to_cluster.get(path) {
                cluster_files
                    .entry(cid)
                    .or_default()
                    .push((path.clone(), src.clone()));
            }
        }
        for primitive in &primitives {
            let mut per_cluster: HashMap<usize, Baseline> = HashMap::new();
            for (cid, files) in &cluster_files {
                if let Some(b) = primitive.fit_cluster_baseline(files, self.language) {
                    per_cluster.insert(*cid, b);
                }
            }
            self.primitive_baselines
                .insert(primitive.name().to_string(), per_cluster);
            self.primitive_fire_count
                .insert(primitive.name().to_string(), 0);
        }
        self.shape_primitives = primitives;
        self
    }

    /// Corpus files that contain `callee` (document frequency; 0 if unseen).
    pub fn callee_file_count(&self, callee: &str) -> usize {
        self.callee_file_counts.get(callee).copied().unwrap_or(0)
    }

    /// Non-data-dominant corpus file count behind the document frequencies.
    pub fn n_corpus_files(&self) -> usize {
        self.n_corpus_files
    }

    /// Whether `path` was one of the fit-time corpus files (repo-relative key,
    /// the same routing key [`Self::contribution_events_impl`] resolves cluster
    /// membership by). A path the model has never seen is a new file — `check`
    /// judges its hunks against the new-file threshold (issue #92).
    pub fn knows_file(&self, path: &Path) -> bool {
        self.file_to_cluster.contains_key(path)
    }

    fn root(callee: &str) -> &str {
        callee.split_once('.').map(|(h, _)| h).unwrap_or(callee)
    }

    /// Dotted callee whose method segment is corpus-known — the receiver
    /// variable is fresh but the invoked method is in-voice.
    fn method_attested(&self, callee: &str) -> bool {
        callee
            .rsplit_once('.')
            .map(|(_, m)| self.attested_methods.contains(m))
            .unwrap_or(false)
    }

    /// Whether an unattested `callee` reaches into a module/namespace foreign
    /// to the repo. It must be *qualified* (carry a receiver/namespace) whose
    /// leading segment is none of: a `self`/`this`-style receiver, a local
    /// binding (a receiver variable the change itself introduced), or a
    /// namespace the corpus already attests. `tokio::runtime::Runtime::new`
    /// and `\Doctrine\ORM\EntityManager.create` are foreign; `String::with_capacity`
    /// (`String` attested), `repository.set_resource` (`repository` local),
    /// and `self.render` are not. A *bare* callee is never namespace-foreign —
    /// a foreign bare call (`event_base_new`) is recognised by its file's
    /// foreign import, not its name.
    pub fn is_namespace_foreign(&self, callee: &str, local: &LocalBindings) -> bool {
        if !is_qualified(callee) {
            return false;
        }
        let lead = leading_namespace(callee);
        if lead.is_empty() || matches!(lead, "self" | "Self" | "this" | "super" | "base" | "cls") {
            return false;
        }
        if local.callables.contains(lead) || local.values.contains(lead) {
            return false;
        }
        !self.attested_namespaces.contains(lead)
    }

    /// Whether `file_source` reaches into a module foreign to the repo. It
    /// does when the file contains an unattested callee that is either
    /// namespace-qualified into an unknown module (`\Doctrine\ORM\X.create`,
    /// `tokio::spawn`) or a **bare** foreign symbol (`event_base_new` — a C
    /// library function; the repo's own new functions are excluded via
    /// `local`, which carries the change's callable and value bindings).
    ///
    /// This is a *file-level* signal so a foreign dependency spread across
    /// hunks — the FQN assignment in one hunk, the calls through a local
    /// receiver in another — flags every hunk of the file's new code, not just
    /// the one line that names the module. A file that only reaches known
    /// modules (`String::with_capacity`, `self.render`, a local receiver) does
    /// not qualify, so the codebase's own new code stays quiet.
    pub fn file_reaches_foreign(&self, file_source: &str, local: &LocalBindings) -> bool {
        non_none_callees(file_source, self.language)
            .iter()
            .any(|c| self.callee_reaches_foreign(c, local))
    }

    /// Whether a single callee reaches into a foreign module. Three cases:
    /// * **module path** (`::` or `\`, e.g. `tokio::spawn`,
    ///   `\React\EventLoop\Loop.get`) — foreign iff its module root is unknown
    ///   to the repo, *regardless* of whether the leaf method is a common
    ///   attested name (`get`). The foreign namespace is the signal.
    /// * **bare symbol** (`event_base_new`) — foreign iff the repo never
    ///   attested it (a C library function); the change's own new functions
    ///   are excluded via `local`.
    /// * **single-`.` receiver form** (`client.newCall`, `Type.Method`) — the
    ///   `.` is ambiguous between a package and a plain member access, so a
    ///   corpus-known method (fresh receiver, in-voice method) is NOT foreign;
    ///   only an unknown method on an unknown, non-local receiver namespace is.
    fn callee_reaches_foreign(&self, callee: &str, local: &LocalBindings) -> bool {
        if local.attests(callee) {
            return false;
        }
        if callee.contains(':') || callee.contains('\\') {
            return self.is_namespace_foreign(callee, local);
        }
        if self.attested.contains(callee) || self.method_attested(callee) {
            return false;
        }
        if is_qualified(callee) {
            self.is_namespace_foreign(callee, local)
        } else {
            true
        }
    }

    fn distinct_unattested_impl(&self, hunk: &str) -> Vec<String> {
        if has_root_error(hunk, self.language) {
            return Vec::new();
        }
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for c in extract_callees(hunk, self.language).into_iter().flatten() {
            if !self.attested.contains(&c) && !seen.contains(&c) {
                seen.insert(c.clone());
                out.push(c);
            }
        }
        out
    }

    pub fn distinct_unattested(&self, hunk: &str) -> Vec<String> {
        self.distinct_unattested_impl(hunk)
    }

    /// [`Self::distinct_unattested`] minus local bindings — the evidence view
    /// matching what the contribution actually counted.
    pub fn distinct_unattested_excluding(
        &self,
        hunk: &str,
        local_bindings: &LocalBindings,
    ) -> Vec<String> {
        self.distinct_unattested_impl(hunk)
            .into_iter()
            .filter(|c| !local_bindings.attests(c) && !self.method_attested(c))
            .collect()
    }

    pub fn count_unattested(&self, hunk: &str) -> usize {
        self.distinct_unattested_impl(hunk).len()
    }

    /// Per-cluster callee counts (for the evidence corpus builder).
    pub fn cluster_callee_counts_for_evidence(&self) -> &HashMap<usize, HashMap<String, usize>> {
        &self.cluster_callee_counts
    }

    /// No cluster logic (`weighted_contribution`). `host_context` is the
    /// parse-error host fallback — see
    /// [`Self::contribution_events_for_file`].
    pub fn weighted_contribution(
        &self,
        hunk: &str,
        alpha: f64,
        root_bonus: f64,
        cap: f64,
        host_context: Option<(&str, usize, usize)>,
        local_bindings: &LocalBindings,
    ) -> f64 {
        let callees: Vec<String> = if has_root_error(hunk, self.language) {
            match host_context {
                Some((host_source, start_line, end_line)) => {
                    callees_in_source_region(host_source, self.language, start_line, end_line)
                }
                None => return 0.0,
            }
        } else {
            non_none_callees(hunk, self.language)
        };
        let mut weights = 0.0;
        let mut seen: HashSet<String> = HashSet::new();
        for c in callees {
            if seen.contains(&c) {
                continue;
            }
            seen.insert(c.clone());
            if local_bindings.attests(&c) {
                continue;
            }
            if self.attested.contains(&c) || self.method_attested(&c) {
                continue;
            }
            if self.attested_roots.contains(Self::root(&c)) {
                weights += alpha + root_bonus;
            } else {
                weights += alpha;
            }
        }
        weights.min(cap)
    }

    /// Per-callee contribution decisions for a hunk against its file's
    /// cluster — the single source of truth behind
    /// [`Self::weighted_contribution_for_file`], also consumed directly by
    /// research scouts and evidence tooling. Does not touch the fire counters.
    ///
    /// `host_context` is `(host_source, hunk_start_line, hunk_end_line)` with
    /// 1-indexed inclusive bounds. It is consulted ONLY when the bare hunk's
    /// parse has root-level errors (parse-error host fallback): callees are then
    /// read from the hunk's region within the host AST. Hunks that parse cleanly
    /// never touch it, so the fallback is purely additive on the
    /// parse-error path (G4.d invariant).
    pub fn contribution_events_for_file(
        &self,
        hunk: &str,
        file_path: Option<&Path>,
        _file_source: Option<&str>,
        host_context: Option<(&str, usize, usize)>,
        local_bindings: &LocalBindings,
    ) -> Vec<ContributionEvent> {
        self.contribution_events_impl(hunk, file_path, host_context, local_bindings, false)
    }

    /// As [`Self::contribution_events_for_file`], but scoring the hunk as though
    /// its file were newly added post-fit (`as_new`): cluster routing is
    /// disabled (a genuinely-new file is never cluster-routed at check time, so
    /// only the global unattested/alpha branches apply) and a callee counts as
    /// attested only when some OTHER corpus file also contains it (`df ≥ 2`) —
    /// exact leave-one-file-out attestation, since a callee whose sole container
    /// is this file would be unattested once the file is removed. This mirrors
    /// how `check` scores a new file and drives the separate new-file
    /// calibration threshold (issue #92 new-file flooding).
    fn contribution_events_impl(
        &self,
        hunk: &str,
        file_path: Option<&Path>,
        host_context: Option<(&str, usize, usize)>,
        local_bindings: &LocalBindings,
        as_new: bool,
    ) -> Vec<ContributionEvent> {
        let callees: Vec<String> = if has_root_error(hunk, self.language) {
            match host_context {
                Some((host_source, start_line, end_line)) => {
                    callees_in_source_region(host_source, self.language, start_line, end_line)
                }
                None => return Vec::new(),
            }
        } else {
            non_none_callees(hunk, self.language)
        };
        if callees.is_empty() {
            return Vec::new();
        }
        // Cluster-conditional branches apply only to files whose cluster
        // membership was FITTED (path lookup). Jaccard-guessing a cluster for
        // an unknown file hands its own staples cluster-absent bonuses when
        // the guess lands wrong (a React file routed into an Effect-heavy
        // cluster) — measured as the dominant FP driver on new-feature
        // commits. Unknown files still get the global unattested branches;
        // `nearest_cluster_for_source` remains for evidence display.
        let cluster_id: Option<usize> = if as_new {
            None
        } else {
            file_path.and_then(|p| self.file_to_cluster.get(p).copied())
        };
        let cluster_set = cluster_id.and_then(|c| self.cluster_attested.get(&c));
        let cluster_counts = cluster_id.and_then(|c| self.cluster_callee_counts.get(&c));

        let mut events = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for c in callees {
            if seen.contains(&c) {
                continue;
            }
            seen.insert(c.clone());
            // Local-binding attestation: a callee the change itself defines,
            // imports from a repo-internal path, or binds as a local value is
            // new code naming its own neighbourhood, not foreign voice.
            if local_bindings.attests(&c) {
                continue;
            }
            let attested_here = if as_new {
                self.callee_file_count(&c) >= 2
            } else {
                self.attested.contains(&c)
            };
            let branch = if !attested_here && !self.method_attested(&c) {
                if self.attested_roots.contains(Self::root(&c)) {
                    Some(ContributionBranch::UnattestedKnownRoot)
                } else {
                    Some(ContributionBranch::Unattested)
                }
            } else if self.attested.contains(&c)
                && cluster_set.map(|s| !s.contains(&c)).unwrap_or(false)
            {
                Some(ContributionBranch::ClusterAbsent)
            } else if self.attested.contains(&c)
                && self.cluster_rare_threshold > 0
                && cluster_id.is_some()
                && cluster_counts.is_some()
                && cluster_counts.unwrap().get(&c).copied().unwrap_or(0)
                    <= self.cluster_rare_threshold
                && self
                    .cluster_sizes
                    .get(&cluster_id.unwrap())
                    .copied()
                    .unwrap_or(0)
                    >= self.cluster_size_min
            {
                Some(ContributionBranch::ClusterRare)
            } else {
                None
            };
            if let Some(branch) = branch {
                events.push(ContributionEvent { callee: c, branch });
            }
        }
        events
    }

    /// Cluster-conditional (`weighted_contribution_for_file`). `host_context`
    /// is the parse-error host fallback — see
    /// [`Self::contribution_events_for_file`].
    #[allow(clippy::too_many_arguments)]
    pub fn weighted_contribution_for_file(
        &mut self,
        hunk: &str,
        file_path: Option<&Path>,
        alpha: f64,
        root_bonus: f64,
        cluster_bonus: f64,
        cap: f64,
        file_source: Option<&str>,
        host_context: Option<(&str, usize, usize)>,
        local_bindings: &LocalBindings,
    ) -> f64 {
        self.hunks_scored += 1;
        let events = self.contribution_events_for_file(
            hunk,
            file_path,
            file_source,
            host_context,
            local_bindings,
        );
        let mut weights = 0.0;
        let mut hunk_fired_rare = false;
        for ev in &events {
            // Rarity weighting scales only the cluster branches; the fire
            // counters keep counting branch *decisions* so the auto-detect
            // probe's fire-rate semantics are unchanged by the weighting.
            let rarity = self
                .rarity_weighting
                .weight(self.callee_file_count(&ev.callee), self.n_corpus_files);
            match ev.branch {
                ContributionBranch::UnattestedKnownRoot => weights += alpha + root_bonus,
                ContributionBranch::Unattested => weights += alpha,
                ContributionBranch::ClusterAbsent => weights += cluster_bonus * rarity,
                ContributionBranch::ClusterRare => {
                    self.rare_branch_fire_count += 1;
                    hunk_fired_rare = true;
                    weights += cluster_bonus * rarity;
                }
            }
        }
        // Shape-primitive dispatch: additive scalars, final cap still bounds
        // the total. Skipped on root-error hunks, matching the Python scorer
        // (the host fallback feeds callee extraction only).
        if !self.shape_primitives.is_empty() && !has_root_error(hunk, self.language) {
            if let Some(cid) = self.cluster_id_for_hunk_file(file_path, file_source) {
                let cluster_size = self.cluster_sizes.get(&cid).copied().unwrap_or(0);
                for i in 0..self.shape_primitives.len() {
                    let name = self.shape_primitives[i].name().to_string();
                    let baseline = self
                        .primitive_baselines
                        .get(&name)
                        .and_then(|m| m.get(&cid));
                    let contribution = self.shape_primitives[i].score(hunk, baseline, cluster_size);
                    if contribution > 0.0 {
                        *self.primitive_fire_count.entry(name).or_insert(0) += 1;
                        weights += contribution;
                    }
                }
            }
        }
        if hunk_fired_rare {
            self.rare_branch_hunks_fired += 1;
        }
        weights.min(cap)
    }

    /// Call-receiver contribution for a hunk scored as though its file were
    /// newly added post-fit (see [`Self::contribution_events_impl`]). Side-effect
    /// free (`&self`, no fire counters, no shape primitives): it feeds only the
    /// separate new-file calibration threshold, which is the honest operating
    /// point for a genuinely-new file — cluster branches are off, so only the
    /// global alpha branches contribute, exactly as `check` scores a new file.
    #[allow(clippy::too_many_arguments)]
    pub fn weighted_contribution_as_new(
        &self,
        hunk: &str,
        file_path: Option<&Path>,
        alpha: f64,
        root_bonus: f64,
        cluster_bonus: f64,
        cap: f64,
        host_context: Option<(&str, usize, usize)>,
        local_bindings: &LocalBindings,
    ) -> f64 {
        let events =
            self.contribution_events_impl(hunk, file_path, host_context, local_bindings, true);
        let mut weights = 0.0;
        for ev in &events {
            let rarity = self
                .rarity_weighting
                .weight(self.callee_file_count(&ev.callee), self.n_corpus_files);
            match ev.branch {
                ContributionBranch::UnattestedKnownRoot => weights += alpha + root_bonus,
                ContributionBranch::Unattested => weights += alpha,
                ContributionBranch::ClusterAbsent | ContributionBranch::ClusterRare => {
                    weights += cluster_bonus * rarity
                }
            }
        }
        weights.min(cap)
    }

    pub fn cluster_id_for_hunk_file(
        &self,
        file_path: Option<&Path>,
        file_source: Option<&str>,
    ) -> Option<usize> {
        if let Some(p) = file_path {
            if let Some(&cid) = self.file_to_cluster.get(p) {
                return Some(cid);
            }
        }
        if let Some(src) = file_source {
            if !self.cluster_attested.is_empty() {
                return self.nearest_cluster_for_source(src).map(|(c, _)| c);
            }
        }
        None
    }

    /// Jaccard-nearest cluster for an arbitrary file source. Ties → smallest
    /// cluster id. None if no clusters or empty callee bag.
    /// (tests for the rarity weighting live at the bottom of this file)
    pub fn nearest_cluster_for_source(&self, file_source: &str) -> Option<(usize, f64)> {
        if self.cluster_attested.is_empty() {
            return None;
        }
        let bag: HashSet<String> = non_none_callees(file_source, self.language)
            .into_iter()
            .collect();
        if bag.is_empty() {
            return None;
        }
        let mut best_cid: Option<usize> = None;
        let mut best_jaccard = -1.0f64;
        let mut cids: Vec<usize> = self.cluster_attested.keys().copied().collect();
        cids.sort_unstable();
        for cid in cids {
            let attested = &self.cluster_attested[&cid];
            let inter = bag.iter().filter(|c| attested.contains(*c)).count();
            let union = bag.len() + attested.len() - inter;
            let jaccard = if union == 0 {
                0.0
            } else {
                inter as f64 / union as f64
            };
            if jaccard > best_jaccard {
                best_jaccard = jaccard;
                best_cid = Some(cid);
            }
        }
        best_cid.map(|c| (c, best_jaccard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::adapters::python::PythonAdapter;

    #[test]
    fn rarity_weight_math() {
        assert_eq!(RarityWeighting::Off.weight(1, 100), 1.0);
        assert_eq!(RarityWeighting::LinearDf.weight(50, 100), 0.5);
        assert_eq!(RarityWeighting::LinearDf.weight(0, 0), 1.0);
        assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(2, 100), 0.0);
        assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(3, 100), 1.0);
        let w = RarityWeighting::LogDf.weight(100, 100);
        assert!(w > 0.99 && w <= 1.0, "log weight near 1 for df=N, got {w}");
        let w1 = RarityWeighting::LogDf.weight(1, 100);
        assert!(w1 < 0.2, "log weight small for df=1, got {w1}");
    }

    fn toy_scorer(weighting: RarityWeighting) -> CallReceiverScorer {
        let adapter = PythonAdapter::new();
        // Two stylistically distinct groups so k-means separates them: the
        // "alpha" files call foo/bar everywhere, the "beta" files call
        // baz/qux. `shared()` appears in exactly one file (globally rare).
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for i in 0..6 {
            files.push((
                PathBuf::from(format!("a{i}.py")),
                "def f():\n    foo()\n    bar()\n".to_string(),
            ));
        }
        for i in 0..6 {
            files.push((
                PathBuf::from(format!("b{i}.py")),
                "def g():\n    baz()\n    qux()\n".to_string(),
            ));
        }
        files.push((
            PathBuf::from("rare.py"),
            "def h():\n    rare_helper()\n".to_string(),
        ));
        CallReceiverScorer::new(&files, Language::Python, 2.0, 5, &adapter, 4, 0, 0, 0)
            .unwrap()
            .with_rarity_weighting(weighting)
    }

    #[test]
    fn df_counts_are_per_file_presence() {
        let cr = toy_scorer(RarityWeighting::Off);
        assert_eq!(cr.n_corpus_files(), 13);
        assert_eq!(cr.callee_file_count("foo"), 6);
        assert_eq!(cr.callee_file_count("rare_helper"), 1);
        assert_eq!(cr.callee_file_count("never_seen"), 0);
    }

    #[test]
    fn as_new_fires_alpha_on_singleton_df_callees() {
        // File-level LOO: a callee whose only corpus container is the held-out
        // file (df == 1) is unattested once that file is treated as newly added,
        // so it fires the global alpha branch; a widely-shared callee (df >= 2)
        // stays attested and, with cluster routing off for new files,
        // contributes nothing. This asymmetry lifts the new-file threshold above
        // the existing-file one (issue #92 new-file flooding).
        let cr = toy_scorer(RarityWeighting::Off);
        // rare_helper has df == 1 (only rare.py) → alpha fires as-new.
        let singleton = cr.weighted_contribution_as_new(
            "rare_helper()\n",
            Some(Path::new("rare.py")),
            2.0,
            2.0,
            5.0,
            5.0,
            None,
            &Default::default(),
        );
        // Alpha fires (>= 2.0), capped at 5.0. Root attestation stays global, so
        // a singleton bare callee resolves to UnattestedKnownRoot (alpha +
        // root_bonus = 4.0) — a bounded, conservative over-estimate that only
        // raises the new-file bar.
        assert!(
            (2.0..=5.0).contains(&singleton),
            "singleton-df callee fires alpha as-new, got {singleton}"
        );
        // foo has df == 6 → attested by other files → no alpha; cluster off → 0.
        let shared = cr.weighted_contribution_as_new(
            "foo()\n",
            Some(Path::new("a0.py")),
            2.0,
            2.0,
            5.0,
            5.0,
            None,
            &Default::default(),
        );
        assert_eq!(
            shared, 0.0,
            "widely-shared callee contributes nothing as-new"
        );
    }

    #[test]
    fn knows_file_tracks_fit_membership() {
        let cr = toy_scorer(RarityWeighting::Off);
        assert!(cr.knows_file(Path::new("a0.py")), "fit file is known");
        assert!(
            !cr.knows_file(Path::new("brand_new.py")),
            "unseen file is a new file"
        );
    }

    #[test]
    fn gated_df_at_one_matches_off_behaviour() {
        // Every globally-attested callee has df >= 1, so GatedDf{min_df: 1}
        // must reproduce the baseline (rule-off) contributions exactly on any hunk.
        let mut off = toy_scorer(RarityWeighting::Off);
        let mut gated = toy_scorer(RarityWeighting::GatedDf { min_df: 1 });
        for hunk in [
            "rare_helper()\nfoo()\n",
            "baz()\n",
            "unknown_callee()\n",
            "foo()\nbar()\nbaz()\nqux()\n",
        ] {
            let a = off.weighted_contribution_for_file(
                hunk,
                Some(Path::new("a0.py")),
                2.0,
                2.0,
                5.0,
                5.0,
                None,
                None,
                &Default::default(),
            );
            let b = gated.weighted_contribution_for_file(
                hunk,
                Some(Path::new("a0.py")),
                2.0,
                2.0,
                5.0,
                5.0,
                None,
                None,
                &Default::default(),
            );
            assert_eq!(a, b, "hunk {hunk:?}");
        }
    }

    #[test]
    fn linear_df_shrinks_rare_callee_cluster_contribution() {
        // `rare_helper` is globally attested (df=1) — in a file from the
        // foo/bar cluster it takes a cluster branch. Under LinearDf its
        // bonus is scaled by 1/13; alpha branches are untouched.
        let mut off = toy_scorer(RarityWeighting::Off);
        let mut lin = toy_scorer(RarityWeighting::LinearDf);
        let hunk = "rare_helper()\n";
        let a = off.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            0.0,
            0.0,
            5.0,
            10.0,
            None,
            None,
            &Default::default(),
        );
        let b = lin.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            0.0,
            0.0,
            5.0,
            10.0,
            None,
            None,
            &Default::default(),
        );
        if a > 0.0 {
            // Cluster branch fired: weighting must shrink it by df/N.
            let expected = a * (1.0 / 13.0);
            assert!(
                (b - expected).abs() < 1e-9,
                "expected {expected}, got {b} (off={a})"
            );
        } else {
            // Cluster assignment put rare.py with a0.py; nothing fired for
            // either mode — invariant still holds.
            assert_eq!(b, 0.0);
        }
    }

    #[test]
    fn parse_error_hunk_falls_back_to_host_region_callees() {
        // A bare `elif` fragment has root-level parse errors in Python.
        let hunk = "elif validate_payload(data):\n    send_alert(data)";
        assert!(has_root_error(hunk, Language::Python));
        let host = "def handler(data):\n    if data is None:\n        return\n    elif validate_payload(data):\n        send_alert(data)\n";
        let callees = callees_in_source_region(host, Language::Python, 4, 5);
        assert_eq!(callees, vec!["validate_payload", "send_alert"]);

        let cr = toy_scorer(RarityWeighting::Off);
        // Without host context: parse error blocks everything (baseline behaviour).
        assert!(cr
            .contribution_events_for_file(
                hunk,
                Some(Path::new("a0.py")),
                None,
                None,
                &Default::default()
            )
            .is_empty());
        // With host context: both (unattested) callees produce events.
        let events = cr.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            Some((host, 4, 5)),
            &Default::default(),
        );
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .all(|e| matches!(e.branch, ContributionBranch::Unattested)));
    }

    #[test]
    fn model_roundtrip_preserves_contribution_decisions() {
        // Export → import must reproduce every contribution decision, both
        // for path-routed hunks (file in a fit-time cluster) and for
        // source-routed hunks (unknown file → Jaccard-nearest cluster).
        let original = toy_scorer(RarityWeighting::Off);
        let model = original.export_model(Path::new(""));
        assert_eq!(model.n_corpus_files, 13);
        assert!(model.attested.contains(&"foo".to_string()));
        let restored =
            CallReceiverScorer::from_model(&model, Language::Python, 2.0, 5, 0, 0).unwrap();

        let unknown_file_source = "def h():\n    baz()\n    qux()\n";
        for hunk in [
            "rare_helper()\nfoo()\n",
            "unknown_callee()\nbaz()\n",
            "foo()\nbar()\nbaz()\nqux()\n",
        ] {
            let a = original.contribution_events_for_file(
                hunk,
                Some(Path::new("a0.py")),
                None,
                None,
                &Default::default(),
            );
            let b = restored.contribution_events_for_file(
                hunk,
                Some(Path::new("a0.py")),
                None,
                None,
                &Default::default(),
            );
            assert_eq!(a.len(), b.len(), "path-routed event count for {hunk:?}");
            for (x, y) in a.iter().zip(b.iter()) {
                assert_eq!(x.callee, y.callee);
                assert_eq!(x.branch, y.branch);
            }
            let a = original.contribution_events_for_file(
                hunk,
                Some(Path::new("never_seen.py")),
                Some(unknown_file_source),
                None,
                &Default::default(),
            );
            let b = restored.contribution_events_for_file(
                hunk,
                Some(Path::new("never_seen.py")),
                Some(unknown_file_source),
                None,
                &Default::default(),
            );
            assert_eq!(a.len(), b.len(), "source-routed event count for {hunk:?}");
            for (x, y) in a.iter().zip(b.iter()) {
                assert_eq!(x.callee, y.callee);
                assert_eq!(x.branch, y.branch);
            }
        }

        // Document frequencies are recovered from the cluster sums.
        assert_eq!(restored.callee_file_count("foo"), 6);
        assert_eq!(restored.callee_file_count("rare_helper"), 1);
        assert_eq!(restored.n_corpus_files(), 13);
    }

    #[test]
    fn host_context_is_ignored_when_bare_hunk_parses() {
        // G4.d invariant: a cleanly-parsing hunk scores identically with and
        // without host context — the fallback is purely additive on the
        // parse-error path.
        let cr = toy_scorer(RarityWeighting::Off);
        let hunk = "foo()\nbar()\n";
        // Deliberately contradictory host region (would yield zero callees).
        let host = "x = 1\ny = 2\n";
        let without = cr.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            None,
            &Default::default(),
        );
        let with_host = cr.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            Some((host, 1, 2)),
            &Default::default(),
        );
        assert_eq!(without.len(), with_host.len());
        for (a, b) in without.iter().zip(with_host.iter()) {
            assert_eq!(a.callee, b.callee);
            assert_eq!(a.branch, b.branch);
        }
    }
}
