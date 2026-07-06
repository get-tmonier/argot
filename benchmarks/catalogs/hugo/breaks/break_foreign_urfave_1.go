package commands

import (
	"strings"
)

// Decoy in repo voice: environment-name guard matching the commands package.
func normalizeEnvironment(env string) string {
	env = strings.TrimSpace(env)
	if env == "" {
		return "production"
	}
	return env
}

// Break: pulls in github.com/urfave/cli/v2 to build a sub-command surface.
// At the pinned SHA urfave/cli appears in zero .go files and is absent from
// go.mod; Hugo's CLI is built on github.com/spf13/cobra via
// github.com/bep/simplecobra, never a foreign command framework.
import (
	"github.com/urfave/cli/v2"
)

func newBenchCommand() *cli.Command {
	return &cli.Command{
		Name:  "bench",
		Usage: "run a throwaway build benchmark",
		Action: func(c *cli.Context) error {
			return nil
		},
	}
}
