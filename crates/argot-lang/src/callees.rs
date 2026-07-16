//! Dotted-callee extraction — per-language call-expression dispatch over a
//! tree-sitter parse. Shared by the base call-receiver scorer
//! (`argot-core::scoring::call_receiver`) and the semantic layer's reinvention
//! confirmation (`argot-rules-semantic`), so "what counts as a call" is one
//! implementation, not two that can drift.

use crate::adapters::Language;
use crate::ts_parse::parse;
use tree_sitter::Node;

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
    crate::adapters::c::is_call_kind(kind)
}

fn extract_c_callee(call_node: Node, src: &[u8]) -> Option<String> {
    crate::adapters::c::callee(call_node, src)
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
        for c in crate::ts_parse::child_nodes(node).into_iter().rev() {
            stack.push(c);
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
        Language::Javascript => ts_call_types as fn(&str) -> bool,
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
        Language::Javascript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
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

/// [`extract_callees`] with the `None` (unmodelled) entries dropped.
pub fn non_none_callees(source: &str, language: Language) -> Vec<String> {
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
pub fn leading_namespace(callee: &str) -> &str {
    let trimmed = callee.trim_start_matches('\\');
    let end = trimmed.find(['.', ':', '\\']).unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// Whether `callee` carries a receiver/namespace qualifier (a `.`, `::`, or
/// `\`). A bare identifier is unqualified.
pub fn is_qualified(callee: &str) -> bool {
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
        Language::Javascript => ts_call_types as fn(&str) -> bool,
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
        Language::Javascript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
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
