package view

import (
	"fmt"
)

// Break: pulls in github.com/alitto/pond worker pool to render PR comment threads through a receiver variable.
import (
	"github.com/alitto/pond"
)

func renderCommentThreads(threads []string, render func(string) error) {
	workers := pond.New(10, 100)
	for _, t := range threads {
		t := t
		workers.Submit(func() {
			_ = render(t)
		})
	}
	workers.StopAndWait()
}

// Decoy in repo voice: plain reviewer helper matching view.go style.
func reviewerStateLabel(state string) string {
	return fmt.Sprintf("(%s)", state)
}
