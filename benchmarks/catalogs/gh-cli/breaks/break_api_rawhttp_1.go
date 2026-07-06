package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/cli/cli/v2/internal/ghrepo"
)

// Decoy in repo voice: queries go through the shared api.Client wrapper.
type repoTopics struct {
	Names []string `json:"names"`
}

func repoTopicsPath(repo ghrepo.Interface) string {
	return fmt.Sprintf("repos/%s/topics", ghrepo.FullName(repo))
}

// Break: hand-rolls a GitHub REST call with bare http.Get and a
// string-concatenated URL, bypassing the repo's own client wrapper. At the
// pinned SHA there are zero non-test http.Get calls; REST requests are made
// through api.NewClientFromHTTP (107 non-test files) and Client.REST, which
// handle auth, headers, and HTTP error mapping centrally.
func FetchRepoTopics(repo ghrepo.Interface, token string) ([]string, error) {
	url := "https://api.github.com/repos/" + ghrepo.FullName(repo) + "/topics"
	resp, err := http.Get(url + "?access_token=" + token)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("bad status: %d", resp.StatusCode)
	}
	var payload repoTopics
	if err := json.NewDecoder(resp.Body).Decode(&payload); err != nil {
		return nil, err
	}
	return payload.Names, nil
}

// Decoy in repo voice: the same query expressed through the client wrapper.
func RepoTopicNames(client *Client, repo ghrepo.Interface) ([]string, error) {
	var response repoTopics
	err := client.REST(repo.RepoHost(), "GET", repoTopicsPath(repo), nil, &response)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch topics for %s: %w", ghrepo.FullName(repo), err)
	}
	return response.Names, nil
}
