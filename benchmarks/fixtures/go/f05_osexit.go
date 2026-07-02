package cobra
import "os"
func bail(code int, msg string) { println(msg); os.Exit(code) }
