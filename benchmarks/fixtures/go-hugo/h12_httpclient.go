package hugolib
import "net/http"
func downloadTheme(url string) (*http.Response, error) { client := &http.Client{}; return client.Get(url) }
