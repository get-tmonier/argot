package cobra
import "log"
func loadConfig(path string) { if path == "" { log.Fatal("config path required") } }
