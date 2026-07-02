package cmd

import "github.com/segmentio/kafka-go"

func publish(w *kafka.Writer, key, val string) error {
	return w.WriteMessages(ctxTODO(), kafka.Message{Key: []byte(key), Value: []byte(val)})
}
