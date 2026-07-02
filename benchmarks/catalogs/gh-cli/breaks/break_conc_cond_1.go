package checks

import (
	"fmt"
	"net/http"
	"sync"

	"github.com/cli/cli/v2/api"
	"github.com/cli/cli/v2/internal/ghrepo"
)

// Decoy in repo voice: fan-out coordinated with channels and wrapped errors.
func fetchCheckSuites(client *http.Client, repo ghrepo.Interface, refs []string) (map[string]int, error) {
	apiClient := api.NewClientFromHTTP(client)
	results := make(map[string]int, len(refs))
	for _, ref := range refs {
		var response struct {
			TotalCount int `json:"total_count"`
		}
		path := fmt.Sprintf("repos/%s/commits/%s/check-suites", ghrepo.FullName(repo), ref)
		if err := apiClient.REST(repo.RepoHost(), "GET", path, nil, &response); err != nil {
			return nil, fmt.Errorf("failed to fetch check suites for %s: %w", ref, err)
		}
		results[ref] = response.TotalCount
	}
	return results, nil
}

// Break: hand-built producer/consumer queue with sync.Cond + sync.Mutex and
// manual Wait/Signal handoff. At the pinned SHA sync.Cond appears in zero
// files; concurrent fan-out is expressed with channels and
// golang.org/x/sync/errgroup (10 non-test files, e.g. pkg/cmd/label/clone.go,
// api/queries_repo.go).
type checkQueue struct {
	mu      sync.Mutex
	cond    *sync.Cond
	pending []string
	closed  bool
}

func newCheckQueue() *checkQueue {
	q := &checkQueue{}
	q.cond = sync.NewCond(&q.mu)
	return q
}

func (q *checkQueue) push(ref string) {
	q.mu.Lock()
	q.pending = append(q.pending, ref)
	q.mu.Unlock()
	q.cond.Signal()
}

func (q *checkQueue) pop() (string, bool) {
	q.mu.Lock()
	defer q.mu.Unlock()
	for len(q.pending) == 0 && !q.closed {
		q.cond.Wait()
	}
	if len(q.pending) == 0 {
		return "", false
	}
	ref := q.pending[0]
	q.pending = q.pending[1:]
	return ref, true
}

func (q *checkQueue) close() {
	q.mu.Lock()
	q.closed = true
	q.mu.Unlock()
	q.cond.Broadcast()
}

// Decoy in repo voice: summary formatting helper.
func summarizeCounts(counts map[string]int) string {
	total := 0
	for _, n := range counts {
		total += n
	}
	return fmt.Sprintf("%d check suites across %d refs", total, len(counts))
}
