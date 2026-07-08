# ID: internal/tableprinter/table_printer.go:58
// NewTablePrinterFor builds a TablePrinter for a writer, TTY flag and terminal width.
func NewTablePrinterFor(w io.Writer, isTTY bool, maxWidth int, cs *iostreams.ColorScheme, headers headerOption) *TablePrinter {
	printer := &TablePrinter{
		TablePrinter: tableprinter.New(w, isTTY, maxWidth),
		isTTY:        isTTY,
		cs:           cs,
	}

	if !isTTY || len(headers.columns) == 0 {
		return printer
	}

	uppercased := make([]string, len(headers.columns))
	for i, col := range headers.columns {
		uppercased[i] = strings.ToUpper(col)
	}

	var padder func(int, string) string
	if cs.Enabled {
		padder = text.PadRight
	}

	printer.AddHeader(
		uppercased,
		WithPadding(padder),
		WithColor(cs.TableHeader),
	)
	return printer
}
