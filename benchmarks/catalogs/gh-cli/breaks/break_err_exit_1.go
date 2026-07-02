package shared

import (
	"fmt"
	"net/http"
	"os"

	"github.com/cli/cli/v2/api"
)

// Decoy in repo voice: library helpers return errors to the caller.
func fetchGistID(client *http.Client, hostname string, gistID string) (string, error) {
	apiClient := api.NewClientFromHTTP(client)
	var response struct {
		ID string `json:"id"`
	}
	if err := apiClient.REST(hostname, "GET", "gists/"+gistID, nil, &response); err != nil {
		return "", fmt.Errorf("failed to fetch gist: %w", err)
	}
	return response.ID, nil
}

// Break: prints the error with fmt.Println and calls os.Exit(1) deep inside a
// shared library helper. At the pinned SHA os.Exit appears only at process
// entry points (cmd/gh/main.go, cmd/gen-docs, the copilot exit-code
// passthrough, the iostreams pager, script/build.go) — never in pkg/cmd
// shared helpers, which return errors up to cobra RunE.
func mustFetchGistFiles(client *http.Client, hostname string, gistID string) map[string]string {
	apiClient := api.NewClientFromHTTP(client)
	var response struct {
		Files map[string]string `json:"files"`
	}
	err := apiClient.REST(hostname, "GET", "gists/"+gistID, nil, &response)
	if err != nil {
		fmt.Println("error fetching gist files:", err)
		os.Exit(1)
	}
	if len(response.Files) == 0 {
		fmt.Println("gist has no files")
		os.Exit(1)
	}
	return response.Files
}

// Decoy in repo voice: caller-facing wrapper that propagates errors.
func GistFilenames(client *http.Client, hostname string, gistID string) ([]string, error) {
	id, err := fetchGistID(client, hostname, gistID)
	if err != nil {
		return nil, fmt.Errorf("could not resolve gist %q: %w", gistID, err)
	}
	names := []string{id}
	return names, nil
}
