package cobra
var requestCounter int
func trackRequest() { requestCounter++; if requestCounter%100 == 0 { println("milestone", requestCounter) } }
