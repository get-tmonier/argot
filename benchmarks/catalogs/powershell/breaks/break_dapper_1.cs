        // Break: fixture spliced at class-member level into engine/Utils.cs.
        // Break: decoy below mirrors the host's manual ADO.NET reader loop; the hunk does not.

        /// <summary>
        /// Reads module registration rows from the local settings store using a plain reader loop.
        /// </summary>
        internal static List<string> ReadModuleRegistrations(System.Data.IDbConnection connection, string query)
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

        // Break: begin hunk — Dapper.SqlMapper query helper replaces the manual reader loop; Dapper
        // Break: is absent from the repo at the pinned SHA — data access here goes through the plain
        // Break: ADO.NET reader pattern above, never a micro-ORM.
        internal static System.Threading.Tasks.Task<IEnumerable<string>> QueryModuleRegistrationsAsync(System.Data.IDbConnection connection, string query)
        {
            return Dapper.SqlMapper.QueryAsync<string>(connection, query);
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied path points under the WSL root share.
        /// </summary>
        internal static bool IsWslPath(string path)
        {
            return path != null && path.StartsWith(WslRootPath, StringComparison.OrdinalIgnoreCase);
        }

