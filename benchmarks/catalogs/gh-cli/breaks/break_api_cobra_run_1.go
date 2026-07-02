package clone

import (
	"flag"
	"fmt"
	"os"

	"github.com/MakeNowJust/heredoc"
	"github.com/cli/cli/v2/pkg/cmdutil"
	"github.com/spf13/cobra"
)

// Decoy in repo voice: RunE + pflag-backed flags on the command object.
func newCmdCloneWiki(f *cmdutil.Factory, runF func(string) error) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "clone-wiki <repository>",
		Short: "Clone the wiki of a repository",
		Args:  cmdutil.ExactArgs(1, "cannot clone wiki: repository argument required"),
		RunE: func(cmd *cobra.Command, args []string) error {
			if runF != nil {
				return runF(args[0])
			}
			return nil
		},
	}
	return cmd
}

// Break: cobra command wired through `Run:` with a hand-rolled stdlib
// flag.FlagSet parse of args and os.Exit on failure. At the pinned SHA every
// command in pkg/cmd uses `RunE:` (221 sites, zero `Run:`) with flags
// registered on cmd.Flags() (spf13/pflag); the stdlib "flag" package is
// imported in zero non-test files.
func newCmdCloneMirror() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "clone-mirror <repository>",
		Short: "Create a mirror clone of a repository",
		Long: heredoc.Doc(`
			Create a bare mirror clone of a repository.
		`),
		Run: func(cmd *cobra.Command, args []string) {
			fs := flag.NewFlagSet("clone-mirror", flag.ContinueOnError)
			depth := fs.Int("depth", 0, "create a shallow clone")
			bare := fs.Bool("bare", true, "create a bare repository")
			if err := fs.Parse(args); err != nil {
				fmt.Println("invalid arguments:", err)
				os.Exit(2)
			}
			if fs.NArg() < 1 {
				fmt.Println("repository argument required")
				os.Exit(2)
			}
			fmt.Printf("cloning %s (depth=%d bare=%v)\n", fs.Arg(0), *depth, *bare)
		},
	}
	return cmd
}

// Decoy in repo voice: flag registration on the cobra command.
func addCloneFlags(cmd *cobra.Command, upstreamName *string) {
	cmd.Flags().StringVarP(upstreamName, "upstream-remote-name", "u", "upstream", "Upstream remote name when cloning a fork")
}
