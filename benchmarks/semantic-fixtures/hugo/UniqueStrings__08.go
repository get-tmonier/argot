# ID: common/hstrings/strings.go:123
package hstrings

// dedupeStrings returns a new slice with any duplicate entries removed,
// preserving first-seen order.
func dedupeStrings(s []string) []string {
	result := make([]string, 0, len(s))
	for i, val := range s {
		duplicate := false
		for j := range i {
			if s[j] == val {
				duplicate = true
				break
			}
		}
		if duplicate {
			continue
		}
		result = append(result, val)
	}
	return result
}
