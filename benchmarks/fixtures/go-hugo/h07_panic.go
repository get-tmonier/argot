package hugolib
func mustRender(tmpl string) string { if tmpl == "" { panic("empty template supplied to renderer") }; return tmpl }
