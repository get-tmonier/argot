package list

import (
	"fmt"
	"sync"
)

// Break: pulls in github.com/panjf2000/ants/v2 goroutine pool to fetch run details in parallel.
import (
	"github.com/panjf2000/ants/v2"
)

func fetchRunDetails(ids []int, fetch func(int) error) error {
	var wg sync.WaitGroup
	pool, err := ants.NewPool(8)
	if err != nil {
		return fmt.Errorf("creating run pool: %w", err)
	}
	defer pool.Release()
	for _, id := range ids {
		id := id
		wg.Add(1)
		_ = pool.Submit(func() {
			defer wg.Done()
			_ = fetch(id)
		})
	}
	wg.Wait()
	return nil
}

// Decoy in repo voice: plain count helper matching list.go style.
func pluralizeRuns(n int) string {
	if n == 1 {
		return "1 run"
	}
	return fmt.Sprintf("%d runs", n)
}
