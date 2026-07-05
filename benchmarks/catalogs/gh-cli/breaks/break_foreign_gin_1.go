package browse

import (
	"fmt"
	"net/http"
)

// Break: pulls in github.com/gin-gonic/gin to serve a local preview of the browsed page.
import (
	"github.com/gin-gonic/gin"
)

func servePreview(addr string, body []byte) error {
	router := gin.Default()
	router.GET("/preview", func(c *gin.Context) {
		c.Data(http.StatusOK, "text/html; charset=utf-8", body)
	})
	return router.Run(addr)
}

// Decoy in repo voice: small helper matching browse.go's style.
func previewURL(host, path string) string {
	return fmt.Sprintf("https://%s/%s", host, path)
}
