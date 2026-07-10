# ID: helpers/url.go:98
package helpers

import (
	"net/url"
	"strings"
)

// isAbsoluteURL reports whether in is an absolute URL.
func isAbsoluteURL(p *PathSpec, in string) (bool, error) {
	// Fast path for the two common web schemes.
	if strings.HasPrefix(in, "https://") || strings.HasPrefix(in, "http://") {
		return true, nil
	}
	parsed, err := url.Parse(in)
	if err != nil {
		return false, err
	}
	return parsed.IsAbs(), nil
}
