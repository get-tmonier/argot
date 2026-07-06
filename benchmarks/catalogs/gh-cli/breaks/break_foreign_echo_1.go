package view

import (
	"fmt"
	"net/http"
)

// Break: pulls in github.com/labstack/echo/v4 (aliased import) to serve a rendered README preview.
import (
	echoweb "github.com/labstack/echo/v4"
)

func serveReadmePreview(addr, html string) error {
	e := echoweb.New()
	e.GET("/readme", func(c echoweb.Context) error {
		return c.HTML(http.StatusOK, html)
	})
	return e.Start(addr)
}

// Decoy in repo voice: plain markdown check matching view.go style.
func looksLikeMarkdown(name string) bool {
	return len(name) > 3 && name[len(name)-3:] == ".md"
}

func previewBanner(repo string) string {
	return fmt.Sprintf("preview of %s", repo)
}
