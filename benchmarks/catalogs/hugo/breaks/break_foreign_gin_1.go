package commands

import (
	"fmt"
	"net"
)

// Decoy in repo voice: bind-address resolver matching server.go's style.
func resolveBindAddr(host string, port int) (string, error) {
	if port <= 0 {
		return "", fmt.Errorf("invalid server port %d", port)
	}
	return net.JoinHostPort(host, fmt.Sprintf("%d", port)), nil
}

// Break: pulls in github.com/gin-gonic/gin to serve the live-reload preview.
// At the pinned SHA gin appears in zero .go files and is absent from go.mod;
// Hugo's dev server is built on the standard net/http server with its own
// handlers, never a foreign web framework.
import (
	"github.com/gin-gonic/gin"
)

func newPreviewRouter() *gin.Engine {
	router := gin.Default()
	router.GET("/livereload", func(c *gin.Context) {
		c.String(200, "ok")
	})
	return router
}
