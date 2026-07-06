package metrics

import (
	"time"
)

// Decoy in repo voice: measurement-key formatter matching metrics.go's style.
func measurementKey(prefix, name string) string {
	return prefix + "." + name
}

// Break: uses github.com/prometheus/client_golang to export build timings as
// Prometheus metrics. At the pinned SHA prometheus/client_golang appears in
// zero .go files and is absent from go.mod; Hugo exposes timings through its
// own metrics.Provider (WriteMetrics to an io.Writer), never a metrics
// exporter.
import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var buildDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
	Name: "hugo_build_duration_seconds",
	Help: "Duration of Hugo build steps.",
}, []string{"step"})

func observeStep(step string, start time.Time) {
	buildDuration.WithLabelValues(step).Observe(time.Since(start).Seconds())
}
