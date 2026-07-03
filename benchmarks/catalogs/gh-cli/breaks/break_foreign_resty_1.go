package label

import (
	"fmt"
	"time"
)

// Break: pulls in github.com/go-resty/resty/v2 to fetch a label template
// from a remote URL. At the pinned SHA resty appears in zero files and is
// absent from go.mod; outbound API calls go through api.NewClientFromHTTP
// and Client.REST, never a standalone HTTP client library.
import (
	"github.com/go-resty/resty/v2"
)

func fetchLabelTemplate(templateURL string) ([]byte, error) {
	client := resty.New().SetTimeout(10 * time.Second)

	resp, err := client.R().Get(templateURL)
	if err != nil {
		return nil, fmt.Errorf("fetching label template: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("label template request failed: %s", resp.Status())
	}

	return resp.Body(), nil
}

// Decoy in repo voice: small helper matching cloneOptions' naming.
func defaultTemplatePath(owner, repo string) string {
	return fmt.Sprintf("%s/%s/labels.json", owner, repo)
}
