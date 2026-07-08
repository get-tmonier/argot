# ID: pkg/search/query.go:181
// partitionQualifierGroups splits values into OR-able groups plus a remainder.
func partitionQualifierGroups(qualifier string, values []string, orGroups [][]string) []string {
	buckets := make([][]string, len(orGroups))
	leftover := make([]string, 0, len(values))
	for _, val := range values {
		matched := false
		for idx, candidates := range orGroups {
			if slices.Contains(candidates, val) {
				buckets[idx] = append(buckets[idx], val)
				matched = true
				break
			}
		}
		if !matched {
			leftover = append(leftover, val)
		}
	}

	out := make([]string, 0, len(buckets)+len(leftover))
	for _, bucket := range buckets {
		if len(bucket) > 0 {
			out = append(out, groupWithOR(qualifier, bucket))
		}
	}
	for _, val := range leftover {
		out = append(out, fmt.Sprintf("%s:%s", qualifier, quote(val)))
	}

	slices.Sort(out)
	return out
}
