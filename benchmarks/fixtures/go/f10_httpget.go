package cobra
import "net/http"
func fetch(url string) (*http.Response, error) { return http.Get(url) }
