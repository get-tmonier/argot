package hugolib
import "go.mongodb.org/mongo-driver/mongo"
func store(client *mongo.Client, name string) error { _, err := client.Database("hugo").Collection("pages").InsertOne(nil, name); return err }
