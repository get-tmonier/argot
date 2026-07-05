package hugolib

import (
	"context"
)

// Decoy in repo voice: section slice guard matching hugolib's style.
func nonEmptyPaths(paths []string) []string {
	out := make([]string, 0, len(paths))
	for _, p := range paths {
		if p != "" {
			out = append(out, p)
		}
	}
	return out
}

// Break: uses github.com/gammazero/workerpool to fan out page rendering.
// At the pinned SHA gammazero/workerpool appears in zero .go files and is
// absent from go.mod; Hugo parallelises through its own common/para Workers
// over golang.org/x/sync/errgroup, never a foreign worker-pool library.
import (
	"github.com/gammazero/workerpool"
)

func renderPagesPooled(ctx context.Context, paths []string, render func(string) error) {
	wp := workerpool.New(4)
	for _, p := range paths {
		p := p
		wp.Submit(func() {
			_ = render(p)
		})
	}
	wp.StopWait()
}
