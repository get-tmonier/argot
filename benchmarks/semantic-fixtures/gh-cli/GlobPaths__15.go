# ID: pkg/cmdutil/args.go:85
// ExpandGlobs expands file patterns into paths, erroring when a pattern matches nothing.
func ExpandGlobs(patterns []string) ([]string, error) {
	resolved := []string{}

	for _, glob := range patterns {
		found, err := filepath.Glob(glob)
		if err != nil {
			return nil, fmt.Errorf("%s: %v", glob, err)
		}
		if len(found) == 0 {
			return []string{}, fmt.Errorf("no matches found for `%s`", glob)
		}
		resolved = append(resolved, found...)
	}

	return resolved, nil
}
