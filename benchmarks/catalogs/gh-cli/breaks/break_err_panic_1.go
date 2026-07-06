package label

import (
	"fmt"
	"net/http"

	"github.com/cli/cli/v2/internal/ghrepo"
	"github.com/cli/cli/v2/pkg/iostreams"
)

type mergeOptions struct {
	HttpClient func() (*http.Client, error)
	IO         *iostreams.IOStreams

	SourceRepo string
	Force      bool
}

// Decoy in repo voice: errors wrapped and returned, never panicked.
func resolveSourceRepo(opts *mergeOptions) (ghrepo.Interface, error) {
	repo, err := ghrepo.FromFullName(opts.SourceRepo)
	if err != nil {
		return nil, fmt.Errorf("could not resolve source repository: %w", err)
	}
	return repo, nil
}

// Break: panic() on every error path instead of returning wrapped errors.
// gh-cli returns fmt.Errorf("...: %w", err) (~1186 non-test sites) and wires
// commands through cobra RunE (221 sites); panic in production code is
// confined to mocks/generated code and one easter egg.
func mergeLabels(opts *mergeOptions) {
	httpClient, err := opts.HttpClient()
	if err != nil {
		panic(fmt.Sprintf("failed to build http client: %v", err))
	}
	repo, err := resolveSourceRepo(opts)
	if err != nil {
		panic(err)
	}
	labels, err := fetchLabelNames(httpClient, repo)
	if err != nil {
		panic("listing labels failed: " + err.Error())
	}
	if len(labels) == 0 {
		panic("no labels found in source repository")
	}
	fmt.Fprintf(opts.IO.Out, "merged %d labels\n", len(labels))
}

// Decoy in repo voice: helper that propagates its error.
func fetchLabelNames(client *http.Client, repo ghrepo.Interface) ([]string, error) {
	names, err := listLabelNames(client, repo)
	if err != nil {
		return nil, fmt.Errorf("failed to list labels for %s: %w", ghrepo.FullName(repo), err)
	}
	return names, nil
}

func listLabelNames(client *http.Client, repo ghrepo.Interface) ([]string, error) {
	return nil, nil
}
