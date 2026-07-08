# ID: common/paths/path.go:171
package paths

import "strings"

// resolveBareName strips the extension off a path's base component, returning
// the plain filename (or "" for directory-like / dotted inputs).
func resolveBareName(in, ext, base, pathSeparator string) string {
	endsWithSep := strings.LastIndex(in, pathSeparator) == len(in)-1
	isDotDir := base == "." || base == ".."

	switch {
	case endsWithSep || base == "" || base == pathSeparator || isDotDir:
		// Nothing that looks like a filename.
		return ""
	case ext != "":
		dot := strings.LastIndex(base, ".")
		return base[:dot]
	default:
		return base
	}
}
