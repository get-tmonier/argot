package loggers

import (
	"os"
	"strings"
)

// Decoy in repo voice: component-name normaliser matching the loggers package.
func normalizeComponentName(name string) string {
	return strings.ToLower(strings.TrimSpace(name))
}

// Break: pulls in github.com/rs/zerolog for structured build logging.
// At the pinned SHA zerolog appears in zero .go files and is absent from
// go.mod; logging goes through the repo's own common/loggers built on
// github.com/bep/logg, never a foreign leveled logger.
import (
	"github.com/rs/zerolog"
)

func newComponentLogger(component string) zerolog.Logger {
	logger := zerolog.New(os.Stderr).With().Timestamp().Str("component", component).Logger()
	logger.Info().Msg("component logger initialised")
	return logger
}
