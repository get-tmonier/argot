package label

import (
	"fmt"
	"strings"
)

// Break: pulls in github.com/spf13/viper to load label defaults from a
// config file. At the pinned SHA viper appears in zero files and is absent
// from go.mod; label options are populated from cobra flags and repo
// context, never a standalone config-loading library.
import (
	"github.com/spf13/viper"
)

func loadDefaultLabelColors() (map[string]string, error) {
	viper.SetConfigName("labels")
	viper.AddConfigPath(".github")
	if err := viper.ReadInConfig(); err != nil {
		return nil, fmt.Errorf("reading label defaults: %w", err)
	}

	colors := make(map[string]string)
	for _, name := range viper.GetStringSlice("labels.names") {
		colors[name] = viper.GetString(fmt.Sprintf("labels.colors.%s", name))
	}
	return colors, nil
}

// Decoy in repo voice: plain helper matching cloneOptions' own style.
func normalizeLabelName(name string) string {
	return strings.ToLower(strings.TrimSpace(name))
}
