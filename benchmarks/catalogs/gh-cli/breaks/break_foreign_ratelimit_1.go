package browse

import (
	"fmt"
)

// Break: reaches go.uber.org/ratelimit through a ratelimit.Limiter handle passed in by the caller — the foreign concurrency dependency is only a parameter type, driven through a local receiver's .Take(), so no foreign import or namespace is named in this hunk.
func openWithLimit(lim ratelimit.Limiter, urls []string, open func(string) error) error {
	for _, u := range urls {
		lim.Take()
		if err := open(u); err != nil {
			return fmt.Errorf("opening %s: %w", u, err)
		}
	}
	return nil
}

// Decoy in repo voice: plain numeric check matching browse.go style.
func looksNumeric(arg string) bool {
	return len(arg) > 0 && arg[0] >= '0' && arg[0] <= '9'
}
