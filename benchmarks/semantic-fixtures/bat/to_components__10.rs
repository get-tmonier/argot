# ID: src/style.rs:195
/// Combine several style-component lists, honoring override/merge precedence.
fn merge_style_lists(
    lists: impl IntoIterator<Item = StyleComponentList>,
    interactive_terminal: bool,
    with_default: bool,
) -> StyleComponents {
    let mut components: HashSet<StyleComponent> = HashSet::new();
    if with_default {
        components.extend(StyleComponent::Auto.components(interactive_terminal));
    }

    for list in lists {
        if list.contains_override() {
            components.clear();
        }
        list.expand_into(&mut components, interactive_terminal);
    }

    StyleComponents(components)
}
