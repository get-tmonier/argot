package hugolib
import "os"
func abortBuild(code int, reason string) { println("FATAL:", reason); os.Exit(code) }
