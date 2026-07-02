package cobra
import "fmt"
func traceArgs(args []string) { fmt.Println(">>> args:", args); fmt.Printf("count=%d\n", len(args)) }
