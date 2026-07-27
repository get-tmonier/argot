// One header, N implementations that must all answer it — a driver layer, a
// storage backend, a cross-platform kernel. Adding a member to the contract
// and forgetting one of them compiles: the missing side only surfaces as a
// link error, or at run time, on the one platform nobody built.
//
// This is the shape no per-file linter can express, because it is about the
// files that did NOT change. `ts_query_old` says what the change added,
// `repo_paths` finds the implementations, `read_repo_file` reads them —
// host API 2.
//
// Adapt IMPLS when you copy this: it is the only line that knows your layout.
const IMPLS = "*/mseguiintf.pas";

// Members declared now, minus members declared before: what this change added.
fn members(text) {
    let out = [];
    if text == () { return out; }
    for line in text.split("\n") {
        let t = line.to_lower();
        t.trim();
        if t.starts_with("function gui_") || t.starts_with("procedure gui_") {
            let rest = t.sub_string(t.index_of("gui_"));
            let name = "";
            for ch in rest.split("") {
                if ch == "(" || ch == ":" || ch == ";" || ch == " " { break; }
                name += ch;
            }
            if name != "" { out.push(name); }
        }
    }
    out
}

let before = members(file.old_text);
let added = [];
for m in members(file.new_text) {
    if !before.contains(m) && !added.contains(m) { added.push(m); }
}
if added.is_empty() { return; }

// Read each implementation once; a member is answered if it is defined there.
let impls = repo_paths(IMPLS);
if impls.is_empty() { return; }
for path in impls {
    let body = read_repo_file(path);
    if body == () { continue; }
    let low = body.to_lower();
    let missing = [];
    for name in added {
        if !low.contains("function " + name) && !low.contains("procedure " + name) {
            missing.push(name);
        }
    }
    if !missing.is_empty() {
        let names = "";
        for n in missing {
            if names != "" { names += ", "; }
            names += n;
        }
        report(1, path + " does not answer " + names + " — every backend that includes this contract must implement it");
    }
}
