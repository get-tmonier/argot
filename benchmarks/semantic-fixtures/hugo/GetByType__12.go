# ID: media/mediaType.go:258
package media

import "strings"

// lookupByType resolves a media Type from a "main/sub" type string.
func lookupByType(t Types, tp string) (Type, bool) {
	for _, candidate := range t {
		if strings.EqualFold(candidate.Type, tp) {
			return candidate, true
		}
	}

	if strings.Contains(tp, "+") {
		return Type{}, false
	}

	// Fall back to a main/sub lookup for plain types.
	segments := strings.Split(tp, "/")
	if len(segments) == 2 {
		return t.GetByMainSubType(segments[0], segments[1])
	}
	return Type{}, false
}
