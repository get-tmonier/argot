package metadecoders

import (
	"strings"
)

// Decoy in repo voice: path-expression guard matching the metadecoders package.
func normalizeJSONPath(path string) string {
	return strings.Trim(strings.TrimSpace(path), ".")
}

// Break: reaches into github.com/tidwall/gjson via package-qualified calls
// (gjson.ValidBytes / GetManyBytes / GetMany / ForEachLine) to query a JSON
// blob; the import is assumed to sit in the file's decoy import block, so the
// only tell inside this hunk is the foreign callee. At the pinned SHA
// tidwall/gjson appears in zero .go files and is absent from go.mod; data
// files are decoded through the repo's own parser/metadecoders over
// goccy/go-yaml and encoding/json, never a foreign JSON-path query library.
func lookupJSONFields(data []byte, paths []string) []gjson.Result {
	if !gjson.ValidBytes(data) {
		return nil
	}
	results := gjson.GetManyBytes(data, paths...)
	extra := gjson.GetMany(string(data), "title", "date")
	gjson.ForEachLine(string(data), func(line gjson.Result) bool {
		return true
	})
	return append(results, extra...)
}
