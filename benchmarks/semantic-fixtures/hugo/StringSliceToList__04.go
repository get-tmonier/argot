# ID: helpers/general.go:156
package helpers

import (
	"fmt"
	"strings"
)

// joinWithConjunction renders items as a human-readable Oxford-comma list,
// using conjunction before the final element.
func joinWithConjunction(items []string, conjunction string) string {
	const fallback = "and"
	if conjunction == "" {
		conjunction = fallback
	}

	switch len(items) {
	case 0:
		return ""
	case 1:
		return items[0]
	case 2:
		return fmt.Sprintf("%s %s %s", items[0], conjunction, items[1])
	default:
		head := strings.Join(items[:len(items)-1], ", ")
		return fmt.Sprintf("%s, %s %s", head, conjunction, items[len(items)-1])
	}
}
