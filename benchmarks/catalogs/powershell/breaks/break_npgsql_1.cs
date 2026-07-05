        // Break: fixture spliced at class-member level into utility/Var.cs.
        // Break: decoy below mirrors the host's WriteObject pipeline emission; the hunk does not.

        /// <summary>
        /// Emits a resolved variable through the pipeline the way this cmdlet already does.
        /// </summary>
        private void EmitVariable(PSVariable variable)
        {
            WriteObject(variable);
        }

        // Break: begin hunk — Npgsql opens a PostgreSQL connection to load persisted variables; Npgsql
        // Break: is absent from the repo at the pinned SHA — this cmdlet reads variables from the
        // Break: session state, never a database. The receiver-variable access masks the leaf .OpenAsync
        // Break: (which collides with attested callees), so the foreign `using` names the dependency.
        using Npgsql;
        private static async System.Threading.Tasks.Task LoadPersistedVariablesAsync(string connectionString)
        {
            var connection = new NpgsqlConnection(connectionString);
            await connection.OpenAsync();
            var command = connection.CreateCommand();
            command.CommandText = "SELECT name, value FROM variables";
        }
        // Break: end hunk

        /// <summary>
        /// True when the variable name matches a reserved automatic variable.
        /// </summary>
        private static bool IsAutomaticVariable(string name)
        {
            return name != null && name.StartsWith("PS");
        }
