package hugolib

import (
	"context"
)

// Decoy in repo voice: worker-count guard matching hugolib's style.
func clampWorkers(n, max int) int {
	if n < 1 {
		return 1
	}
	if n > max {
		return max
	}
	return n
}

// Break: reaches into github.com/marusama/semaphore/v2 through a receiver
// variable — semaphore.New(...) binds a foreign Semaphore, then every use is
// sem.Acquire(...) / sem.Release(...). The constructor's leaf method (New)
// collides with the repo's own pervasive New(), and Acquire/Release go through
// the local receiver sem, so no callee names a foreign namespace: a genuinely
// masked foreign concurrency primitive that may not fire. At the pinned SHA
// marusama/semaphore appears in zero .go files and is absent from go.mod;
// Hugo bounds parallelism through its own common/para Workers over
// golang.org/x/sync/errgroup, never a foreign weighted-semaphore library.
func renderBounded(ctx context.Context, tasks []func() error, maxWeight int) error {
	sem := semaphore.New(maxWeight)
	for _, task := range tasks {
		task := task
		if err := sem.Acquire(ctx, 1); err != nil {
			return err
		}
		go func() {
			defer sem.Release(1)
			_ = task()
		}()
	}
	return nil
}
