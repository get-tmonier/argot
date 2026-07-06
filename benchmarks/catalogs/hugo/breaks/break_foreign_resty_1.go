package create

import (
	"strings"
)

// Decoy in repo voice: remote-path guard matching the create package.
func trimRemoteScheme(pathname string) string {
	pathname = strings.TrimPrefix(pathname, "https://")
	return strings.TrimPrefix(pathname, "http://")
}

// Break: uses github.com/go-resty/resty/v2 to fetch a remote resource.
// At the pinned SHA go-resty/resty appears in zero .go files and is absent
// from go.mod; Hugo fetches remote resources through its own httpcache and
// the standard net/http client, never a foreign HTTP client library.
import (
	"github.com/go-resty/resty/v2"
)

func fetchRemoteResource(url string) ([]byte, error) {
	client := resty.New()
	resp, err := client.R().Get(url)
	if err != nil {
		return nil, err
	}
	return resp.Body(), nil
}
