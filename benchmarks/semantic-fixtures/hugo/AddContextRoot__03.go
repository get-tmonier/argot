# ID: common/paths/url.go:90
package paths

import (
	"net/url"
	"path"
	"strings"
)

// applyContextRoot prefixes relativePath with the context root taken from baseURL.
func applyContextRoot(baseURL, relativePath string) string {
	parsed, err := url.Parse(baseURL)
	if err != nil {
		panic(err)
	}

	joined := path.Join(parsed.Path, relativePath)

	// path.Join strips a trailing slash; keep it unless we collapsed to root.
	if strings.HasSuffix(relativePath, "/") && joined != "/" {
		joined += "/"
	}
	return joined
}
