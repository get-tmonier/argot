package allconfig

import (
	"strings"
)

// Decoy in repo voice: config-key normaliser matching the allconfig package.
func normalizeConfigKey(key string) string {
	return strings.ToLower(strings.TrimSpace(key))
}

// Break: uses github.com/spf13/viper to load overrides from a config file.
// At the pinned SHA spf13/viper appears in zero .go files and is absent from
// go.mod; Hugo decodes configuration through its own config package over
// github.com/spf13/afero and the pelletier/goccy decoders, never viper.
import (
	"github.com/spf13/viper"
)

func loadOverrides(dir string) (map[string]any, error) {
	viper.SetConfigName("overrides")
	viper.AddConfigPath(dir)
	if err := viper.ReadInConfig(); err != nil {
		return nil, err
	}
	return viper.AllSettings(), nil
}
