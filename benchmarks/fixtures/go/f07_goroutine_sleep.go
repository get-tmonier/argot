package cobra
import "time"
func pollUntil(done chan bool) { go func() { for { time.Sleep(100 * time.Millisecond); select { case <-done: return; default: } } }() }
