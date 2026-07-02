package cmd

import "github.com/prometheus/client_golang/prometheus"

var cmdRuns = prometheus.NewCounter(prometheus.CounterOpts{Name: "gh_cmd_runs_total"})

func recordRun() {
	cmdRuns.Inc()
}
