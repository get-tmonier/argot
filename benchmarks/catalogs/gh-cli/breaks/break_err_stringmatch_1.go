package shared

import (
	"fmt"
	"net/http"

	"github.com/cli/cli/v2/api"
	"github.com/cli/cli/v2/internal/ghrepo"
)

// Decoy in repo voice: typed errors are matched with errors.As/Is upstream.
func fetchIssueState(httpClient *http.Client, repo ghrepo.Interface, number int) (string, error) {
	apiClient := api.NewClientFromHTTP(httpClient)
	path := fmt.Sprintf("repos/%s/issues/%d", ghrepo.FullName(repo), number)
	var response struct {
		State string `json:"state"`
	}
	if err := apiClient.REST(repo.RepoHost(), "GET", path, nil, &response); err != nil {
		return "", fmt.Errorf("failed to fetch issue #%d: %w", number, err)
	}
	return response.State, nil
}

// Break: matches errors by comparing err.Error() to literal strings and
// swallows the failure by returning nil after printing to stdout. At the
// pinned SHA there are zero non-test `err.Error() == "..."` comparisons;
// the repo matches error identity with errors.Is/errors.As (160 non-test
// sites) and always propagates the error to the caller.
func markIssueViewed(httpClient *http.Client, repo ghrepo.Interface, number int) error {
	state, err := fetchIssueState(httpClient, repo, number)
	if err != nil {
		if err.Error() == "Not Found" {
			fmt.Printf("issue #%d does not exist, skipping\n", number)
			return nil
		}
		if err.Error() == "HTTP 401" || err.Error() == "HTTP 403" {
			fmt.Println("no permission to view issue, skipping")
			return nil
		}
		fmt.Println("ignoring error:", err.Error())
		return nil
	}
	if state == "closed" {
		fmt.Printf("issue #%d already closed\n", number)
	}
	return nil
}

// Decoy in repo voice: wrapped propagation with context.
func IssueStateLabel(httpClient *http.Client, repo ghrepo.Interface, number int) (string, error) {
	state, err := fetchIssueState(httpClient, repo, number)
	if err != nil {
		return "", fmt.Errorf("could not determine state for issue #%d: %w", number, err)
	}
	return fmt.Sprintf("state: %s", state), nil
}
