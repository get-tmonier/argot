package run

import (
	"fmt"
	"io/ioutil"
	"net/http"
	"path/filepath"

	"github.com/cli/cli/v2/pkg/iostreams"
)

// Decoy in repo voice: modern os/io file APIs with wrapped errors.
type workflowFileOptions struct {
	IO       *iostreams.IOStreams
	HTTP     func() (*http.Client, error)
	Filename string
}

// Break: deprecated io/ioutil APIs (ioutil.ReadFile / ioutil.ReadAll /
// ioutil.WriteFile). At the pinned SHA the repo imports io/ioutil in zero
// files; the same operations are done with os.ReadFile (22 non-test sites)
// and io.ReadAll (65 non-test sites).
func readWorkflowInputs(opts *workflowFileOptions) ([]byte, error) {
	yamlContent, err := ioutil.ReadFile(opts.Filename)
	if err != nil {
		return nil, fmt.Errorf("could not read workflow file: %w", err)
	}
	client, err := opts.HTTP()
	if err != nil {
		return nil, err
	}
	resp, err := client.Get("https://example.invalid/schema")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	schema, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("could not read schema: %w", err)
	}
	cache := filepath.Join(filepath.Dir(opts.Filename), ".schema-cache")
	if err := ioutil.WriteFile(cache, schema, 0o644); err != nil {
		return nil, fmt.Errorf("could not cache schema: %w", err)
	}
	return yamlContent, nil
}

// Decoy in repo voice: small helper with wrapped error.
func workflowDisplayName(opts *workflowFileOptions) (string, error) {
	base := filepath.Base(opts.Filename)
	if base == "" {
		return "", fmt.Errorf("invalid workflow filename %q", opts.Filename)
	}
	return base, nil
}
