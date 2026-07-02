package cobra
import "github.com/sirupsen/logrus"
func handleReq(c *Command) error { logrus.WithField("cmd", c.Name()).Info("run"); return nil }
