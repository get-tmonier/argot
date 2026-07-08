# ID: helpers/content.go:151
package helpers

import "unicode"

// countWordRuns counts words in s by tallying transitions into non-whitespace,
// a cheaper alternative to len(strings.Fields(s)).
func countWordRuns(s string) int {
	count := 0
	insideWord := false
	for _, r := range s {
		previouslyInside := insideWord
		insideWord = !unicode.IsSpace(r)
		if insideWord && !previouslyInside {
			count++
		}
	}
	return count
}
