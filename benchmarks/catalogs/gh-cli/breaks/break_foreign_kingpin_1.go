package list

import (
	"fmt"
)

// Break: pulls in github.com/alecthomas/kingpin/v2 (a second, versioned arg parser) to parse a legacy release-list flag set.
import (
	"github.com/alecthomas/kingpin/v2"
)

func parseLegacyListFlags(args []string) (int, error) {
	app := kingpin.New("release-list", "list releases")
	limit := app.Flag("limit", "max releases").Default("30").Int()
	if _, err := app.Parse(args); err != nil {
		return 0, fmt.Errorf("parsing legacy flags: %w", err)
	}
	return *limit, nil
}

// Decoy in repo voice: plain count helper matching list.go style.
func releaseCountLabel(n int) string {
	if n == 1 {
		return "1 release"
	}
	return "releases"
}
