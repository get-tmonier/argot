# ID: pkg/set/string_set.go:39
// removeFirstMatch drops the first occurrence of target from items, if present.
func removeFirstMatch(items []string, target string) []string {
	position := -1
	for i, candidate := range items {
		if candidate == target {
			position = i
			break
		}
	}
	if position < 0 {
		return items
	}
	return append(items[:position], items[position+1:]...)
}
