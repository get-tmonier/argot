# ID: pkg/search/query.go:146
// IssueAdvancedQueryString renders a query in advanced issue search syntax.
func IssueAdvancedQueryString(q Query) string {
	terms := q.ImmutableKeywords
	if terms == "" {
		terms = strings.Join(formatKeywords(q.Keywords), " ")
	}
	filters := strings.Join(formatQualifiers(q.Qualifiers, formatAdvancedIssueSearch), " ")

	switch {
	case filters == "" && terms == "":
		return ""
	case filters != "" && terms != "":
		// Bracket the keywords so their operators (notably OR) cannot leak.
		return fmt.Sprintf("( %s ) %s", terms, filters)
	case terms != "":
		return terms
	default:
		return filters
	}
}
