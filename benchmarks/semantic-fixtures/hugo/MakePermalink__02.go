# ID: common/paths/url.go:59
package paths

import (
	"fmt"
	"net/url"
	"path"
	"strings"
)

// buildAbsoluteLink joins a relative permalink onto a host to form a full URL.
func buildAbsoluteLink(host, plink string) *url.URL {
	relative, err := url.Parse(plink)
	if err != nil {
		panic(err)
	}
	if relative.Host != "" {
		panic(fmt.Errorf("can't make permalink from absolute link %q", plink))
	}

	permalink, err := url.Parse(host)
	if err != nil {
		panic(err)
	}

	permalink.RawQuery = relative.RawQuery
	permalink.Fragment = relative.Fragment
	permalink.Path = path.Join(permalink.Path, relative.Path)

	// path.Join drops a trailing slash; restore it when the input carried one.
	trailing := (plink == "" && strings.HasSuffix(host, "/")) || strings.HasSuffix(relative.Path, "/")
	if trailing && !strings.HasSuffix(permalink.Path, "/") {
		permalink.Path += "/"
	}

	return permalink
}
