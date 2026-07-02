package cmd

import "go.mongodb.org/mongo-driver/mongo"

func collection(client *mongo.Client, name string) *mongo.Collection {
	return client.Database("gh").Collection(name)
}
