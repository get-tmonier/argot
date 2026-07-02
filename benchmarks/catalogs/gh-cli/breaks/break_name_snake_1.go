package git

import (
	"context"
	"fmt"
	"strings"
)

// Decoy in repo voice: MixedCaps names, small focused methods on Client.
func (c *Client) CurrentBranchName(ctx context.Context) (string, error) {
	args := []string{"symbolic-ref", "--quiet", "HEAD"}
	cmd, err := c.Command(ctx, args...)
	if err != nil {
		return "", err
	}
	out, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to read current branch: %w", err)
	}
	return strings.TrimPrefix(strings.TrimSpace(string(out)), "refs/heads/"), nil
}

// Break: snake_case function and variable names (parse_remote_line,
// remote_name, fetch_url, push_url, result_map). At the pinned SHA there are
// zero snake_case function definitions in non-generated Go code; the repo
// uses Go MixedCaps throughout (e.g. CurrentBranchName, parseRemotes,
// UncommittedChangeCount).
func parse_remote_line(raw_line string) (string, string, bool) {
	trimmed_line := strings.TrimSpace(raw_line)
	field_list := strings.Fields(trimmed_line)
	if len(field_list) < 2 {
		return "", "", false
	}
	remote_name := field_list[0]
	remote_url := field_list[1]
	return remote_name, remote_url, true
}

func collect_remote_urls(raw_output string) map[string]string {
	result_map := make(map[string]string)
	line_list := strings.Split(raw_output, "\n")
	for _, raw_line := range line_list {
		remote_name, remote_url, ok := parse_remote_line(raw_line)
		if !ok {
			continue
		}
		if _, seen_before := result_map[remote_name]; !seen_before {
			result_map[remote_name] = remote_url
		}
	}
	return result_map
}

// Decoy in repo voice: MixedCaps helper mirroring repo naming.
func normalizeRemoteURL(remoteURL string) string {
	normalized := strings.TrimSuffix(remoteURL, ".git")
	return strings.TrimSpace(normalized)
}
