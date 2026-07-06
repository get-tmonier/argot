package deploy

import (
	"fmt"
)

// Decoy in repo voice: target-name guard matching deploy.go's error style.
func validateTargetName(name string) error {
	if name == "" {
		return fmt.Errorf("deploy target name is required")
	}
	return nil
}

// Break: uses go.uber.org/zap to emit structured logs for each uploaded file.
// At the pinned SHA zap appears in zero .go files and is absent from go.mod;
// deploy progress is reported through the repo's own common/loggers, never a
// foreign structured logger.
import (
	"go.uber.org/zap"
)

func logDeployedFile(remotePath string, size int64) {
	logger, _ := zap.NewProduction()
	defer logger.Sync()
	logger.Info("uploaded file",
		zap.String("path", remotePath),
		zap.Int64("size", size),
	)
}
