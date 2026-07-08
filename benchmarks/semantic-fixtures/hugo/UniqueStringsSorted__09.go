# ID: common/hstrings/strings.go:163
package hstrings

import "sort"

// sortedUnique sorts s in place and removes duplicates, returning the deduped prefix.
func sortedUnique(s []string) []string {
	if len(s) == 0 {
		return nil
	}

	ordered := sort.StringSlice(s)
	ordered.Sort()

	last := 0
	for cur := 1; cur < len(s); cur++ {
		if !ordered.Less(last, cur) {
			continue
		}
		last++
		s[last] = s[cur]
	}

	return s[:last+1]
}
