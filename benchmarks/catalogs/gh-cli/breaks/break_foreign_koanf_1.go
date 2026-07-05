package config

import (
	"fmt"
)

// Break: reaches github.com/knadh/koanf through a *koanf.Koanf handle passed in by the caller — the foreign config dependency is only a receiver type and is read through .String/.Get, method names the repo attests, so no foreign import or namespace is named in this hunk.
func mergeHostDefaults(k *koanf.Koanf, host string) map[string]string {
	out := make(map[string]string)
	out["git_protocol"] = k.String(host + ".git_protocol")
	if v := k.Get(host + ".editor"); v != nil {
		if s, ok := v.(string); ok {
			out["editor"] = s
		}
	}
	return out
}

// Decoy in repo voice: plain host-key join matching config.go style.
func hostScopedKey(host, key string) string {
	return fmt.Sprintf("%s.%s", host, key)
}
