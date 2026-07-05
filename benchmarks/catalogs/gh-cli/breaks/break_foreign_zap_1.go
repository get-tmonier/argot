package api

import (
	"fmt"
)

// Break: pulls in go.uber.org/zap for structured request logging in the api package.
import (
	"go.uber.org/zap"
)

func logGraphQLQuery(queryName string, variables map[string]interface{}) {
	logger, _ := zap.NewProduction()
	defer logger.Sync()
	logger.Info("graphql query",
		zap.String("query", queryName),
		zap.Int("variables", len(variables)),
	)
}

// Decoy in repo voice: plain error wrapper matching api package style.
func wrapQueryError(queryName string, err error) error {
	return fmt.Errorf("running %s query: %w", queryName, err)
}
