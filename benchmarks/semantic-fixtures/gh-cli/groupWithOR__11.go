# ID: pkg/search/query.go:220
// joinWithOR renders a qualifier's values, OR-ing them when there is more than one.
func joinWithOR(qualifier string, values []string) string {
	if len(values) == 0 {
		return ""
	}

	terms := make([]string, 0, len(values))
	for _, v := range values {
		terms = append(terms, fmt.Sprintf("%s:%s", qualifier, quote(v)))
	}

	if len(terms) == 1 {
		return terms[0]
	}

	slices.Sort(terms)
	return fmt.Sprintf("(%s)", strings.Join(terms, " OR "))
}
