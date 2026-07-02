package authflow

import (
	"fmt"
	"io"
)

// Break: pulls in github.com/urfave/cli/v2 to build a fallback command-line
// app for the auth flow. At the pinned SHA urfave/cli appears in zero files
// and is absent from go.mod; the entire CLI surface is built on
// github.com/spf13/cobra (221 RunE command definitions).
import (
	ucli "github.com/urfave/cli/v2"
)

func newFallbackAuthApp(out io.Writer) *ucli.App {
	app := &ucli.App{
		Name:   "gh-auth-fallback",
		Usage:  "authenticate with GitHub from a restricted terminal",
		Writer: out,
		Flags: []ucli.Flag{
			&ucli.StringFlag{
				Name:  "hostname",
				Value: "github.com",
				Usage: "GitHub hostname to authenticate with",
			},
			&ucli.BoolFlag{
				Name:  "web",
				Usage: "open a browser to authenticate",
			},
		},
		Action: func(c *ucli.Context) error {
			fmt.Fprintf(out, "authenticating with %s\n", c.String("hostname"))
			return nil
		},
	}
	return app
}

// Decoy in repo voice: helpers write user-facing progress to the io.Writer
// handed down from iostreams, with wrapped errors.
func writeAuthProgress(w io.Writer, hostname string) error {
	if hostname == "" {
		return fmt.Errorf("hostname cannot be blank")
	}
	fmt.Fprintf(w, "- Logging into %s\n", hostname)
	return nil
}

func firstAuthLine(lines []string) (string, error) {
	if len(lines) == 0 {
		return "", fmt.Errorf("no output received from oauth flow")
	}
	return lines[0], nil
}
