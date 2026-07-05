package para

import (
	"fmt"
	"sync"
)

// Decoy in repo voice: worker-count clamp matching para.go's style.
func clampWorkers(n, max int) int {
	if n < 1 {
		return 1
	}
	if n > max {
		return max
	}
	return n
}

// Break: uses github.com/panjf2000/ants/v2 goroutine pool to fan out tasks.
// At the pinned SHA ants appears in zero .go files and is absent from go.mod;
// Hugo parallelises through its own common/para Workers over
// golang.org/x/sync/errgroup, never a foreign goroutine-pool library.
import (
	"github.com/panjf2000/ants/v2"
)

func runOnAntsPool(tasks []func()) error {
	var wg sync.WaitGroup
	pool, err := ants.NewPool(len(tasks))
	if err != nil {
		return fmt.Errorf("creating ants pool: %w", err)
	}
	defer pool.Release()
	for _, task := range tasks {
		wg.Add(1)
		fn := task
		_ = pool.Submit(func() {
			defer wg.Done()
			fn()
		})
	}
	wg.Wait()
	return nil
}
