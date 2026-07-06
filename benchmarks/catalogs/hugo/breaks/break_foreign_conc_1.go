package hugolib

import (
	"context"
)

// Decoy in repo voice: section slice guard matching site.go's style.
func nonEmptySections(sections []string) []string {
	out := make([]string, 0, len(sections))
	for _, s := range sections {
		if s != "" {
			out = append(out, s)
		}
	}
	return out
}

// Break: uses github.com/sourcegraph/conc worker pool to render sections
// concurrently. At the pinned SHA conc appears in zero .go files and is absent
// from go.mod; Hugo renders in parallel through its own common/para Workers
// over golang.org/x/sync/errgroup, never a foreign structured-concurrency lib.
import (
	"github.com/sourcegraph/conc/pool"
)

func renderSectionsConcurrently(ctx context.Context, sections []string, render func(string) error) error {
	p := pool.New().WithMaxGoroutines(4).WithContext(ctx).WithFirstError()
	for _, section := range sections {
		s := section
		p.Go(func(context.Context) error {
			return render(s)
		})
	}
	return p.Wait()
}
