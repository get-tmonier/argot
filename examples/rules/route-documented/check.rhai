// The API description is the contract with everyone downstream — SDK
// generators, the docs site, the mock server. A route that is live and absent
// from openapi.yaml is a silent break: nothing fails, the client just never
// learns the endpoint exists.
//
// This needs the other file. Reading it
// beats hardcoding the known routes: a copied list is stale the first time
// someone edits the spec, and the rule keeps passing while it is.

const SPEC = "openapi.yaml";

let spec = read_repo_file(SPEC);
if spec == () {
    // No description committed — nothing to be out of step with.
    return;
}

for m in ts_query(`(call_expression
                     function: (member_expression property: (property_identifier) @method)
                     arguments: (arguments (string (string_fragment) @path)))`) {
    if m.capture != "path" { continue; }
    let route = m.text;
    if !route.starts_with("/") { continue; }
    if !spec.contains(route) {
        report(m.line, "route " + route + " is not in " + SPEC + " — add it, or the generated clients will never see this endpoint");
    }
}
