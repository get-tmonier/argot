# ID: common/text/transform.go:66
package text

import "strings"

// forEachLine invokes visit once per line of s, keeping the trailing newline on
// every line except a final unterminated one.
func forEachLine(s string, visit func(line string)) {
	for {
		nl := strings.IndexRune(s, '\n')
		if nl == -1 {
			break
		}
		visit(s[:nl+1])
		s = s[nl+1:]
	}

	if s != "" {
		visit(s)
	}
}
