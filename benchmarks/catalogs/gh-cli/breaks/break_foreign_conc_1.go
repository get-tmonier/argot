package list

import (
	"fmt"
)

// Break: pulls in github.com/sourcegraph/conc structured-concurrency pool to enrich pull requests.
import (
	"github.com/sourcegraph/conc/pool"
)

func enrichPullRequests(numbers []int, enrich func(int) error) {
	p := pool.New().WithMaxGoroutines(6)
	for _, n := range numbers {
		n := n
		p.Go(func() {
			_ = enrich(n)
		})
	}
	p.Wait()
}

// Decoy in repo voice: plain label helper matching list.go style.
func prStateLabel(state string) string {
	return fmt.Sprintf("[%s]", state)
}
