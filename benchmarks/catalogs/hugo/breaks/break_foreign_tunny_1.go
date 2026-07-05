package images

import (
	"image"
)

// Decoy in repo voice: dimension guard matching the images package.
func nonZeroDimensions(imgs []image.Image) int {
	count := 0
	for _, img := range imgs {
		if img != nil && img.Bounds().Dx() > 0 {
			count++
		}
	}
	return count
}

// Break: reaches into github.com/Jeffail/tunny via a package-qualified
// constructor (tunny.NewFunc) to build a fixed worker pool for image resizing;
// the import is assumed to sit in the file's decoy import block, so the only
// tell inside this hunk is the foreign callee. At the pinned SHA Jeffail/tunny
// appears in zero .go files and is absent from go.mod; image work is fanned
// out through the repo's own common/para Workers over golang.org/x/sync/
// errgroup, never a foreign goroutine-pool library.
func resizePooled(imgs []image.Image, resize func(image.Image) image.Image) []image.Image {
	out := make([]image.Image, len(imgs))
	pool := tunny.NewFunc(4, func(payload any) any {
		i := payload.(int)
		return resize(imgs[i])
	})
	defer pool.Close()
	for i := range imgs {
		out[i] = pool.Process(i).(image.Image)
	}
	return out
}
