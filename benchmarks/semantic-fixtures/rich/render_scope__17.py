# ID: rich/scope.py:14
def render_variables(
    scope: "Mapping[str, Any]",
    *,
    title: Optional[TextType] = None,
    sort_keys: bool = True,
    indent_guides: bool = False,
    max_length: Optional[int] = None,
    max_string: Optional[int] = None,
    max_depth: Optional[int] = None,
    overflow: Optional["OverflowMethod"] = None,
) -> "ConsoleRenderable":
    """Render a mapping of variable names to values as a titled panel."""
    highlighter = ReprHighlighter()
    variables_table = Table.grid(padding=(0, 1), expand=False)
    variables_table.add_column(justify="right")

    def dunder_first(item: Tuple[str, Any]) -> Tuple[bool, str]:
        """Sort special (dunder) variables first, then alphabetically."""
        name, _ = item
        return (not name.startswith("__"), name.lower())

    entries = sorted(scope.items(), key=dunder_first) if sort_keys else scope.items()
    for name, value in entries:
        name_text = Text.assemble(
            (name, "scope.key.special" if name.startswith("__") else "scope.key"),
            (" =", "scope.equals"),
        )
        variables_table.add_row(
            name_text,
            Pretty(
                value,
                highlighter=highlighter,
                indent_guides=indent_guides,
                max_length=max_length,
                max_string=max_string,
                max_depth=max_depth,
                overflow=overflow,
            ),
        )
    return Panel.fit(
        variables_table,
        title=title,
        border_style="scope.border",
        padding=(0, 1),
    )
