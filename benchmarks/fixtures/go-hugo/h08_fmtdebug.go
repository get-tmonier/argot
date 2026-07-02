package hugolib
import "fmt"
func debugPage(path string) { fmt.Println("### RENDERING", path); fmt.Printf("path length = %d\n", len(path)) }
