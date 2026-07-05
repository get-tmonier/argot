        // Break: fixture spliced at class-member level into Library/LibraryManager.cs.
        // Break: decoy below mirrors the host's own EF Core item read; the hunk does not.

        /// <summary>
        /// Counts stored items through the repository the way this manager already reads
        /// from the database — via the EF Core-backed item repository.
        /// </summary>
        private int CountItems(BaseItemKind kind)
        {
            return _itemRepository.GetCount(new InternalItemsQuery { IncludeItemTypes = new[] { kind } });
        }

        // Break: begin hunk — Npgsql NpgsqlConnection/NpgsqlCommand run a raw Postgres query through a
        // Break: receiver variable. Npgsql is 0-usage in the repo at the pinned SHA (no `using Npgsql;`
        // Break: here — the tell is the bare NpgsqlConnection type) — all reads go through EF Core.
        private static long CountItemsInPostgres(string connectionString, int kind)
        {
            var connection = new NpgsqlConnection(connectionString);
            connection.Open();
            var command = new NpgsqlCommand("SELECT COUNT(*) FROM items WHERE kind = @kind", connection);
            command.Parameters.Add(new NpgsqlParameter("kind", kind));
            return (long)command.ExecuteScalar()!;
        }
        // Break: end hunk

        /// <summary>
        /// True when the given kind should be excluded from user-facing counts.
        /// </summary>
        private static bool IsHiddenKind(BaseItemKind kind)
            => kind == BaseItemKind.Folder;
