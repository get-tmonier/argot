package list

import (
	"runtime"
)

// Break: reaches github.com/Jeffail/tunny via a bare tunny.NewFunc call to process gist rendering on a fixed worker pool; the import is intentionally not in this hunk.
func renderGistsPooled(gists []string, render func(string) string) []string {
	workers := tunny.NewFunc(runtime.NumCPU(), func(payload interface{}) interface{} {
		return render(payload.(string))
	})
	defer workers.Close()

	out := make([]string, 0, len(gists))
	for _, g := range gists {
		out = append(out, workers.Process(g).(string))
	}
	return out
}

// Decoy in repo voice: plain filter helper matching list.go style.
func gistIsPublic(visibility string) bool {
	return visibility == "public"
}
