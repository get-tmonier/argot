        // Break: fixture spliced at class-member level into engine/Utils.cs.
        // Break: decoy below mirrors the host's plain ADO.NET reader loop; the hunk does not.

        /// <summary>
        /// Reads settings rows from the local store using the plain reader pattern this file uses.
        /// </summary>
        internal static List<string> ReadSettings(System.Data.IDbConnection connection, string query)
        {
            var results = new List<string>();
            using (System.Data.IDbCommand command = connection.CreateCommand())
            {
                command.CommandText = query;
                using (System.Data.IDataReader reader = command.ExecuteReader())
                {
                    while (reader.Read())
                    {
                        results.Add(reader.GetString(0));
                    }
                }
            }

            return results;
        }

        // Break: begin hunk — MongoDB.Driver document client reads module settings; MongoDB.Driver is
        // Break: absent from the repo at the pinned SHA — persistence here is plain ADO.NET. The client
        // Break: is constructed fully qualified, then reached through a receiver variable.
        internal static long CountModuleDocuments(string connectionString, string database)
        {
            var client = new MongoDB.Driver.MongoClient(connectionString);
            var db = client.GetDatabase(database);
            var collection = db.GetCollection<object>("modules");
            return collection.CountDocuments(MongoDB.Driver.Builders<object>.Filter.Empty);
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied path points under the WSL root share.
        /// </summary>
        internal static bool IsWslPathLocal(string path)
        {
            return path != null && path.StartsWith(WslRootPath, StringComparison.OrdinalIgnoreCase);
        }
