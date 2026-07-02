package com.google.common.argotfix;

import com.mongodb.client.MongoClient;
import com.mongodb.client.MongoDatabase;

public class J08Mongo {
    public MongoDatabase db(MongoClient c) { return c.getDatabase("guava"); }
}
