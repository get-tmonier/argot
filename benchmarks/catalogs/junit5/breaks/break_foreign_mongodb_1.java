/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

/**
 * Internal helper that reads stored execution records from a document store.
 */
final class ExecutionRecordStore {

	private ExecutionRecordStore() {
		/* no-op */
	}

	static String describe(String database, String collection) {
		Preconditions.notBlank(database, "database must not be blank");
		return database + "." + collection;
	}

	// Break: MongoDB driver — com.mongodb is 0-usage in junit5 at the pinned SHA
	// (git grep com.mongodb over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 has no persistence layer and holds
	// execution state in its own in-memory containers, never a foreign document
	// store.
	static long countFailed(String uri, String database, String collection) {
		com.mongodb.client.MongoClient client = com.mongodb.client.MongoClients.create(uri);
		com.mongodb.client.MongoDatabase db = client.getDatabase(database);
		com.mongodb.client.MongoCollection<org.bson.Document> records = db.getCollection(collection);
		records.createIndex(com.mongodb.client.model.Indexes.ascending("uniqueId"));
		return records.countDocuments(com.mongodb.client.model.Filters.eq("status", "failed"));
	}
}
