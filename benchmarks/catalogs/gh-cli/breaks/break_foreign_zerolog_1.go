package cmdutil

import (
	"fmt"
	"os"
)

// Break: pulls in github.com/rs/zerolog for structured command logging. At
// the pinned SHA zerolog appears in zero files and is absent from go.mod;
// user-facing output goes through iostreams
// (fmt.Fprintf(opts.IO.ErrOut, ...)) and errors are returned as values, not
// logged by a leveled logger.
import (
	"github.com/rs/zerolog"
)

var cmdLogger = zerolog.New(os.Stderr).With().Timestamp().Str("component", "cmdutil").Logger()

func logCommandFailure(commandPath string, err error) {
	cmdLogger.Error().
		Err(err).
		Str("command", commandPath).
		Msg("command execution failed")
}

func logCommandStart(commandPath string, argCount int) {
	cmdLogger.Info().
		Str("command", commandPath).
		Int("args", argCount).
		Msg("command started")
}

// Decoy in repo voice: sentinel-style error helper mirroring pkg/cmdutil.
type FlagUsageError struct {
	message string
}

func (e FlagUsageError) Error() string {
	return e.message
}

func NewFlagUsageError(format string, args ...interface{}) FlagUsageError {
	return FlagUsageError{message: fmt.Sprintf(format, args...)}
}
