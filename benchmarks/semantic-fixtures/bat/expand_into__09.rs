# ID: src/style.rs:163
/// Fold this list's +/-/override actions into an accumulating component set.
fn apply_component_actions(
    list: &StyleComponentList,
    components: &mut HashSet<StyleComponent>,
    interactive_terminal: bool,
) {
    for (action, component) in list.0.iter() {
        let subcomponents = component.components(interactive_terminal);
        if let ComponentAction::Remove = action {
            components.retain(|c| !subcomponents.contains(c));
        } else {
            components.extend(subcomponents);
        }
    }
}
