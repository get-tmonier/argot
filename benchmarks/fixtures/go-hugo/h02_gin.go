package hugolib
import "github.com/gin-gonic/gin"
func serve() { r := gin.Default(); r.GET("/", func(c *gin.Context) { c.String(200, "ok") }); r.Run() }
