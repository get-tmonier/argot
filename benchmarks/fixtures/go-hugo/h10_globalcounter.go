package hugolib
var pagesRendered int
func countPage() { pagesRendered++; if pagesRendered%500 == 0 { println("rendered", pagesRendered) } }
