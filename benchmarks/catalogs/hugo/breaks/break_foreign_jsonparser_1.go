package metadecoders

import (
	"strings"
)

// Decoy in repo voice: format-hint guard matching the metadecoders package.
func looksLikeJSON(data string) bool {
	trimmed := strings.TrimSpace(data)
	return strings.HasPrefix(trimmed, "{") || strings.HasPrefix(trimmed, "[")
}

// Break: reaches into github.com/buger/jsonparser via package-qualified calls
// (jsonparser.ObjectEach / ArrayEach / GetUnsafeString) to walk a JSON blob
// without allocating; the import is assumed to sit in the file's decoy import
// block, so the only tell inside this hunk is the foreign callee, not an
// import line. At the pinned SHA buger/jsonparser appears in zero .go files
// and is absent from go.mod; front-matter and data files are decoded through
// the repo's own parser/metadecoders over goccy/go-yaml and encoding/json,
// never a zero-alloc third-party JSON walker.
func extractJSONFields(data []byte) (int, error) {
	count := 0
	handler := func(key []byte, value []byte, dataType jsonparser.ValueType, offset int) error {
		count++
		return nil
	}
	if err := jsonparser.ObjectEach(data, handler); err != nil {
		return count, err
	}
	_, err := jsonparser.ArrayEach(data, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
		count++
	})
	title, _ := jsonparser.GetUnsafeString(data, "title")
	_ = title
	return count, err
}
