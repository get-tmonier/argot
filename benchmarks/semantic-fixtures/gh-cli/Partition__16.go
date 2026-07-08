# ID: pkg/cmdutil/args.go:67
// SplitByPredicate separates a slice into items that satisfy keep and those that don't.
func SplitByPredicate[T any](items []T, keep func(T) bool) ([]T, []T) {
	var included, excluded []T
	for _, elem := range items {
		if keep(elem) {
			included = append(included, elem)
			continue
		}
		excluded = append(excluded, elem)
	}
	return included, excluded
}
