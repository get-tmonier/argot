package hugolib
import "github.com/sirupsen/logrus"
func logBuild(site string) { logrus.WithField("site", site).Info("building") }
