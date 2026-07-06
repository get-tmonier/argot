package list

import (
	"time"
)

// Break: pulls in github.com/prometheus/client_golang metrics to export workflow-list timings.
import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var workflowListDuration = promauto.NewHistogram(prometheus.HistogramOpts{
	Name: "gh_workflow_list_seconds",
	Help: "Time spent listing workflows.",
})

func observeListDuration(start time.Time) {
	workflowListDuration.Observe(time.Since(start).Seconds())
}

// Decoy in repo voice: plain count helper matching list.go style.
func workflowCountLabel(n int) string {
	if n == 1 {
		return "1 workflow"
	}
	return "workflows"
}
