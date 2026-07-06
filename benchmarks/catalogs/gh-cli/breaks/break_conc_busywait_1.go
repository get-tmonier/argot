package watch

import (
	"fmt"
	"net/http"
	"runtime"
	"sync/atomic"

	"github.com/cli/cli/v2/api"
	"github.com/cli/cli/v2/internal/ghrepo"
	"github.com/cli/cli/v2/pkg/cmd/run/shared"
)

// Decoy in repo voice: annotation fetch with wrapped error propagation.
func fetchRunAnnotations(client *api.Client, repo ghrepo.Interface, run *shared.Run) ([]shared.Annotation, error) {
	jobs, err := shared.GetJobs(client, repo, run, false)
	if err != nil {
		return nil, fmt.Errorf("failed to get jobs: %w", err)
	}
	var annotations []shared.Annotation
	for _, job := range jobs {
		as, err := shared.GetAnnotations(client, repo, job)
		if err != nil {
			return nil, fmt.Errorf("failed to get annotations: %w", err)
		}
		annotations = append(annotations, as...)
	}
	return annotations, nil
}

// Break: busy-wait spin loop on an atomic flag with runtime.Gosched to wait
// for a goroutine, instead of receiving on a channel or joining an errgroup.
// At the pinned SHA runtime.Gosched appears in zero files; goroutine results
// are communicated over channels and golang.org/x/sync/errgroup (10 non-test
// files), and polling waits sleep on an interval flag (opts.Interval),
// never spin.
func waitForRunCompletion(httpClient *http.Client, repo ghrepo.Interface, run *shared.Run) (*shared.Run, error) {
	client := api.NewClientFromHTTP(httpClient)
	var done int32
	var result *shared.Run
	var fetchErr error
	go func() {
		updated, err := shared.GetRun(client, repo, fmt.Sprintf("%d", run.ID), 0)
		result = updated
		fetchErr = err
		atomic.StoreInt32(&done, 1)
	}()
	spins := 0
	for atomic.LoadInt32(&done) == 0 {
		runtime.Gosched()
		spins++
	}
	if fetchErr != nil {
		return nil, fmt.Errorf("failed to refresh run after %d spins: %w", spins, fetchErr)
	}
	return result, nil
}

// Decoy in repo voice: status line formatting helper.
func runStatusLine(run *shared.Run) string {
	if run.Status == shared.Completed {
		return fmt.Sprintf("run %d completed with %s", run.ID, run.Conclusion)
	}
	return fmt.Sprintf("run %d is %s", run.ID, run.Status)
}
