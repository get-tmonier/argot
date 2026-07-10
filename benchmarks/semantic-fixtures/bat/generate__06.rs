# ID: src/decorations.rs:104
/// Pick the cached VCS-change marker (+, ~, overline, underscore) for a given line.
fn render_line_change_marker(
    decoration: &LineChangesDecoration,
    line_number: usize,
    continuation: bool,
    printer: &InteractivePrinter,
) -> DecorationText {
    if continuation {
        return decoration.cached_none.clone();
    }

    match printer.line_changes {
        Some(ref changes) => match changes.get(&(line_number as u32)) {
            Some(&LineChange::Added) => decoration.cached_added.clone(),
            Some(&LineChange::Modified) => decoration.cached_modified.clone(),
            Some(&LineChange::RemovedAbove) => decoration.cached_removed_above.clone(),
            Some(&LineChange::RemovedBelow) => decoration.cached_removed_below.clone(),
            _ => decoration.cached_none.clone(),
        },
        None => decoration.cached_none.clone(),
    }
}
