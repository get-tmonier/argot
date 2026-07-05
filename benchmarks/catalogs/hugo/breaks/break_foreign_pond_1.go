package resources

import (
	"fmt"
)

// Decoy in repo voice: dimension guard matching image.go's error style.
func validDimensions(w, h int) error {
	if w <= 0 || h <= 0 {
		return fmt.Errorf("invalid image dimensions %dx%d", w, h)
	}
	return nil
}

// Break: uses github.com/alitto/pond worker pool to resize images in parallel.
// At the pinned SHA pond appears in zero .go files and is absent from go.mod;
// Hugo processes images through its own resource cache and common/para Workers
// over golang.org/x/sync/errgroup, never a foreign worker-pool library.
import (
	"github.com/alitto/pond"
)

func resizeAllOnPool(paths []string, resize func(string) error) {
	pool := pond.New(10, 1000)
	defer pool.StopAndWait()
	for _, path := range paths {
		p := path
		pool.Submit(func() {
			_ = resize(p)
		})
	}
}
