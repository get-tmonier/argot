# ID: config/env.go:47
package config

import (
	"os"

	"github.com/pbnjay/memory"
)

// resolveMemoryCeiling returns the upper memory limit in bytes for Hugo's caches,
// honouring HUGO_MEMORYLIMIT (in GB) or defaulting to a quarter of system memory.
func resolveMemoryCeiling() uint64 {
	if mem := os.Getenv("HUGO_MEMORYLIMIT"); mem != "" {
		if configured := stringToGibabyte(mem); configured > 0 {
			return configured
		}
	}

	// Reserve a quarter of the total system memory when nothing is set.
	total := memory.TotalMemory()
	if total == 0 {
		return 2 * gigabyte
	}
	return uint64(total / 4)
}
