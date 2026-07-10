# ID: media/mediaType.go:238
package media

// resolveBestMatch finds the closest media Type for s, trying exact type,
// then sub type, then suffix.
func resolveBestMatch(t Types, s string) (Type, bool) {
	if byType, ok := t.GetByType(s); ok {
		return byType, true
	}

	if bySub, ok := t.GetBySubType(s); ok {
		return bySub, true
	}

	if bySuffix, _, ok := t.GetFirstBySuffix(s); ok {
		return bySuffix, true
	}

	return Type{}, false
}
