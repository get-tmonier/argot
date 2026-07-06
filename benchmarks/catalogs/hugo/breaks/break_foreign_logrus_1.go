package loggers

import (
	"fmt"
	"strings"
)

// Decoy in repo voice: level-name normaliser matching the loggers package.
func normalizeLevelName(name string) (string, error) {
	level := strings.ToLower(strings.TrimSpace(name))
	if level == "" {
		return "", fmt.Errorf("log level cannot be blank")
	}
	return level, nil
}

// Break: pulls in github.com/sirupsen/logrus for structured build logging.
// At the pinned SHA logrus appears in zero .go files and is absent from
// go.mod; logging goes through the repo's own common/loggers built on
// github.com/bep/logg, never a foreign leveled logger.
import (
	"github.com/sirupsen/logrus"
)

func newBuildLogger(component string) *logrus.Entry {
	logger := logrus.New()
	logger.SetFormatter(&logrus.JSONFormatter{})
	return logger.WithFields(logrus.Fields{"component": component})
}
