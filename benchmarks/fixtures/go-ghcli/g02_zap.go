package cmd

import "go.uber.org/zap"

func logRequest(path string, status int) {
	logger, _ := zap.NewProduction()
	logger.Info("request", zap.String("path", path), zap.Int("status", status))
}
