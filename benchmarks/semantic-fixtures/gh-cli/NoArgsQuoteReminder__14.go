# ID: pkg/cmdutil/args.go:38
// RejectStrayArgs errors on any positional args, nudging toward quoting values.
func RejectStrayArgs(cmd *cobra.Command, args []string) error {
	if len(args) == 0 {
		return nil
	}

	message := fmt.Sprintf("unknown argument %q", args[0])
	if len(args) > 1 {
		message = fmt.Sprintf("unknown arguments %q", args)
	}

	sawValueFlag := false
	cmd.Flags().Visit(func(flag *pflag.Flag) {
		if flag.Value.Type() != "bool" {
			sawValueFlag = true
		}
	})

	if sawValueFlag {
		message += "; please quote all values that have spaces"
	}
	return FlagErrorf("%s", message)
}
