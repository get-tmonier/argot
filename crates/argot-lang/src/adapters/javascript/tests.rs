use super::*;

#[test]
fn import_bindings_pairs_foreign_bound_names_with_modules() {
    let adapter = JavaScriptAdapter::new();
    let src = "import express from \"express\";\n\
               import { render, h as hyper } from \"preact\";\n\
               import { helper } from \"./util\";\n";
    let b: std::collections::HashSet<(String, String)> =
        adapter.import_bindings(src).into_iter().collect();
    assert!(
        b.contains(&("express".to_string(), "express".to_string())),
        "{b:?}"
    );
    assert!(
        b.contains(&("render".to_string(), "preact".to_string())),
        "{b:?}"
    );
    assert!(
        b.contains(&("hyper".to_string(), "preact".to_string())),
        "{b:?}"
    );
    // Relative import stays repo-internal.
    assert!(!b.iter().any(|(n, _)| n == "helper"), "{b:?}");
}

#[test]
fn callable_bodies_covers_declarations_methods_and_arrow_consts() {
    let adapter = JavaScriptAdapter::new();
    let src = "\
function parseHeader(line) {
  const [k, v] = line.split(':');
  return { key: k.trim(), value: v.trim() };
}

const buildUrl = (base, path) => {
  return base.replace(/\\/$/, '') + '/' + path;
};

class Router {
  addRoute(method, handler) {
this.routes.push({ method, handler });
return this;
  }
}
";
    let bodies = adapter.callable_bodies(src);
    let names: Vec<&str> = bodies.iter().map(|b| b.symbol.as_str()).collect();
    assert!(names.contains(&"parseHeader"), "fn decl: {names:?}");
    assert!(names.contains(&"buildUrl"), "arrow const: {names:?}");
    assert!(names.contains(&"addRoute"), "class method: {names:?}");
    // Line ranges are 1-indexed and span the whole body.
    let ph = bodies.iter().find(|b| b.symbol == "parseHeader").unwrap();
    assert_eq!(ph.start_line, 1);
    assert_eq!(ph.end_line, 4);
}

#[test]
fn callable_bodies_covers_commonjs_prototype_assignment() {
    let adapter = JavaScriptAdapter::new();
    let src = "\
res.status = function status(code) {
  this.statusCode = code;
  return this;
};

app.prototype.render = function (name, options, callback) {
  const cache = this.cache;
  return cache[name] || callback();
};

exports.compile = (src) => {
  const ast = parse(src);
  return ast;
};

defineGetter(req, 'host', function host() {
  const forwarded = this.get('X-Forwarded-Host');
  return forwarded || this.get('Host');
});

defineGetter(req, 'fresh', function () {
  const method = this.method;
  return method === 'GET' && checkFresh(this.headers);
});
";
    let names: Vec<String> = adapter
        .callable_bodies(src)
        .into_iter()
        .map(|b| b.symbol)
        .collect();
    // Named function expression keeps its own name.
    assert!(names.contains(&"status".to_string()), "{names:?}");
    // Anonymous RHS takes the target's last property.
    assert!(names.contains(&"render".to_string()), "{names:?}");
    // Arrow assigned to `exports.compile`.
    assert!(names.contains(&"compile".to_string()), "{names:?}");
    // Named getter function expression keeps its own name; each real
    // function is emitted exactly once (no double-count from the arms).
    assert_eq!(
        names.iter().filter(|n| *n == "host").count(),
        1,
        "{names:?}"
    );
    // Anonymous getter takes the sibling string-literal property name.
    assert!(names.contains(&"fresh".to_string()), "{names:?}");
}

#[test]
fn transpiled_output_is_detected_as_auto_generated() {
    let adapter = JavaScriptAdapter::new();
    // esbuild/bundler sourceMappingURL trailer (survives outside dist/).
    let bundled = "function load(x) {\n  return { id: x.length };\n}\nexport { load };\n//# sourceMappingURL=out.js.map\n";
    assert!(
        adapter.is_auto_generated(bundled, &crate::test_support::generated_markers()),
        "sourceMappingURL trailer"
    );
    // tsc CommonJS __esModule interop banner (present even without sourcemaps).
    let tsc = "\"use strict\";\nObject.defineProperty(exports, \"__esModule\", { value: true });\nexports.load = load;\nfunction load(x) { return x.length; }\n";
    assert!(
        adapter.is_auto_generated(tsc, &crate::test_support::generated_markers()),
        "tsc __esModule banner"
    );
    // Hand-authored JS with neither tell stays authored voice.
    let authored = "const express = require('express');\nfunction route(req, res) {\n  res.json({ ok: true });\n}\nmodule.exports = route;\n";
    assert!(
        !adapter.is_auto_generated(authored, &crate::test_support::generated_markers()),
        "hand-written JS"
    );
}

#[test]
fn require_and_esm_and_dynamic_imports_are_captured() {
    let adapter = JavaScriptAdapter::new();
    let src = concat!(
        "const foo = require('foo');\n",
        "const y = require(\"bar/baz\");\n",
        "import a from 'react';\n",
        "import {x} from './local.js';\n",
        "export * from 'lib';\n",
        "const m = await import('m');\n",
    );
    let imports = adapter.extract_imports(src);
    // CommonJS require (single- and double-quoted, verbatim subpath).
    assert!(imports.contains("foo"));
    assert!(imports.contains("bar/baz"));
    // ESM default import.
    assert!(imports.contains("react"));
    // ESM re-export from a bare module.
    assert!(imports.contains("lib"));
    // Dynamic import.
    assert!(imports.contains("m"));
    // Relative specifiers are dropped.
    assert!(!imports.contains("./local.js"));
    assert_eq!(
        imports,
        ["foo", "bar/baz", "react", "lib", "m"]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<_>>()
    );
}

#[test]
fn namespace_and_named_imports_are_captured() {
    let adapter = JavaScriptAdapter::new();
    let imports = adapter.extract_imports(
        "import * as ns from 'lodash';\nimport { readFile } from 'node:fs';\nimport 'side-effect';\n",
    );
    assert!(imports.contains("lodash"));
    // Specifiers are kept verbatim, never split on ':' or '/'.
    assert!(imports.contains("node:fs"));
    assert!(imports.contains("side-effect"));
}

#[test]
fn relative_require_is_dropped() {
    let adapter = JavaScriptAdapter::new();
    let imports = adapter.extract_imports("const local = require('./util');\n");
    assert!(imports.is_empty());
}

#[test]
fn extract_imports_with_spans_underline_the_specifier() {
    let adapter = JavaScriptAdapter::new();
    let spans = adapter.extract_imports_with_spans("import a from 'react';\n");
    assert_eq!(spans.len(), 1);
    let (spec, line, col_start, col_end) = &spans[0];
    assert_eq!(spec, "react");
    assert_eq!(*line, 1);
    // "react" starts one column past the opening quote.
    assert_eq!(col_end - col_start, "react".len());
}

#[test]
fn adapter_reports_javascript_language() {
    let adapter = JavaScriptAdapter::new();
    assert_eq!(adapter.language(), Language::Javascript);
    assert_eq!(adapter.line_comment_prefix(), "//");
}

#[test]
fn identifier_noise_contains_reserved_words() {
    let adapter = JavaScriptAdapter::new();
    assert!(adapter.identifier_noise().contains("const"));
    assert!(adapter.identifier_noise().contains("arguments"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}

#[test]
fn subpath_imports_are_internal_not_foreign() {
    let adapter = JavaScriptAdapter::new();
    let src = "import { cmd } from '#internal/commands';\nimport z from 'zod';\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("zod"));
    assert!(!imports.iter().any(|s| s.starts_with('#')), "{imports:?}");
    let bindings = adapter.internal_import_bindings(src);
    assert!(bindings.contains("cmd"), "{bindings:?}");
}

#[test]
fn pnpm_workspace_packages_resolve_as_internal() {
    let dir = std::env::temp_dir().join(format!("argot_js_pnpm_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("packages/core")).unwrap();
    std::fs::write(dir.join("package.json"), "{\"name\": \"@acme/repo\"}\n").unwrap();
    std::fs::write(
        dir.join("pnpm-workspace.yaml"),
        "packages:\n  - \"packages/*\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("packages/core/package.json"),
        "{\"name\": \"@acme/core\"}\n",
    )
    .unwrap();

    let modules = JavaScriptAdapter::new().resolve_repo_modules(&dir);
    assert!(modules.exact.contains("@acme/core"), "{:?}", modules.exact);
    let _ = std::fs::remove_dir_all(&dir);
}
