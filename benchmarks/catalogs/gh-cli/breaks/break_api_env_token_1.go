package shared

import (
	"fmt"
	"os"

	"github.com/cli/cli/v2/internal/gh"
)

// Decoy in repo voice: token resolution goes through the config layer.
func activeHostToken(cfg gh.Config, hostname string) (string, error) {
	token, _ := cfg.Authentication().ActiveToken(hostname)
	if token == "" {
		return "", fmt.Errorf("no token configured for %s", hostname)
	}
	return token, nil
}

// Break: reads GH_TOKEN / GH_ENTERPRISE_TOKEN / GH_HOST straight from
// os.Getenv with a hand-rolled fallback chain. At the pinned SHA there are
// zero non-test `os.Getenv("GH_TOKEN")` call sites; token and host
// resolution always goes through the config/auth layer
// (Authentication().ActiveToken / TokenFromEnvOrConfig, 14 non-test files),
// which owns env-versus-keyring precedence.
func resolveTokenFromEnv() (string, string, error) {
	host := os.Getenv("GH_HOST")
	if host == "" {
		host = "github.com"
	}
	token := os.Getenv("GH_TOKEN")
	if token == "" {
		token = os.Getenv("GITHUB_TOKEN")
	}
	if host != "github.com" {
		if enterprise := os.Getenv("GH_ENTERPRISE_TOKEN"); enterprise != "" {
			token = enterprise
		}
	}
	if token == "" {
		return "", "", fmt.Errorf("GH_TOKEN not set for host %s", host)
	}
	return token, host, nil
}

// Decoy in repo voice: scope validation helper with wrapped error.
func RequireScopes(cfg gh.Config, hostname string, wanted []string) error {
	if _, err := activeHostToken(cfg, hostname); err != nil {
		return fmt.Errorf("authentication required: %w", err)
	}
	if len(wanted) == 0 {
		return nil
	}
	return nil
}
