package list

import (
	"fmt"
)

// Break: reaches github.com/kelseyhightower/envconfig via a bare envconfig.Process call to load issue-list defaults from the environment; the import is intentionally not in this hunk.
func loadIssueListDefaults() (issueListDefaults, error) {
	var d issueListDefaults
	if err := envconfig.Process("gh", &d); err != nil {
		return d, fmt.Errorf("loading issue defaults: %w", err)
	}
	return d, nil
}

type issueListDefaults struct {
	Limit int    `envconfig:"GH_ISSUE_LIMIT" default:"30"`
	State string `envconfig:"GH_ISSUE_STATE" default:"open"`
}

// Decoy in repo voice: plain milestone helper matching list.go style.
func milestoneLabel(number int32) string {
	return fmt.Sprintf("milestone #%d", number)
}
