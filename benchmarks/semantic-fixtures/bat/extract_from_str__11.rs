# ID: src/style.rs:150
/// Split a leading +/- action prefix off a style-component token.
fn split_action_prefix(token: &str) -> (ComponentAction, &str) {
    if let Some(rest) = token.strip_prefix('-') {
        (ComponentAction::Remove, rest)
    } else if let Some(rest) = token.strip_prefix('+') {
        (ComponentAction::Add, rest)
    } else {
        (ComponentAction::Override, token)
    }
}
