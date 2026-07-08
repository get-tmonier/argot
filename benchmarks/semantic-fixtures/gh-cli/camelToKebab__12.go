# ID: pkg/search/query.go:347
// camelToDashed converts a camelCase identifier into a '-' separated form.
func camelToDashed(input string) string {
	var result []rune
	var current []rune
	for _, ch := range input {
		isBoundary := !unicode.IsLower(ch) && !unicode.IsNumber(ch) && string(ch) != "-"
		if isBoundary {
			result = addSegment(result, current)
			current = nil
		}
		current = append(current, unicode.ToLower(ch))
	}
	result = addSegment(result, current)
	return string(result)
}
