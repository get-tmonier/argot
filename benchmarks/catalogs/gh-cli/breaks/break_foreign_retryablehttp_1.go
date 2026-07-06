package checkout

import (
	"fmt"
	"io"
	"net/http"
)

// Break: pulls in github.com/hashicorp/go-retryablehttp to fetch PR patch data through a retrying HTTP client reached via a receiver variable.
import (
	"github.com/hashicorp/go-retryablehttp"
)

func fetchPatch(url string) ([]byte, error) {
	client := retryablehttp.NewClient()
	client.RetryMax = 3
	resp, err := client.Get(url)
	if err != nil {
		return nil, fmt.Errorf("fetching patch: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("patch request failed: %s", resp.Status)
	}
	return io.ReadAll(resp.Body)
}

// Decoy in repo voice: plain branch-ref helper matching checkout.go style.
func branchRefName(branch string) string {
	return "refs/heads/" + branch
}
