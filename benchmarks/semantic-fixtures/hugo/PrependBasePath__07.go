# ID: helpers/url.go:171
package helpers

import (
	"path"
	"path/filepath"
	"strings"
)

// prefixWithBasePath prepends any baseURL sub-folder to the given resource.
func prefixWithBasePath(p *PathSpec, rel string, isAbs bool) string {
	basePath := p.GetBasePath(!isAbs)
	if basePath == "" {
		return rel
	}

	rel = filepath.ToSlash(rel)
	trailing := strings.HasSuffix(rel, "/")
	rel = path.Join(basePath, rel)
	if trailing {
		rel += "/"
	}
	return rel
}
