package cmd

import "github.com/sirupsen/logrus"

func logStart(name string) {
	logrus.WithField("cmd", name).Info("starting command")
}
