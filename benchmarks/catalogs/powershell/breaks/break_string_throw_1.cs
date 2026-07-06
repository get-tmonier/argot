        // Break: fixture spliced at class-member level into engine/Utils.cs.
        // Break: hunk mixes bare string throws with int return codes; the repo throws typed
        // Break: PS exceptions via PSTraceSource.New*Exception and never signals via -1/0 codes.

        /// <summary>
        /// The configuration file name used when probing module directories.
        /// </summary>
        internal const string ConfigFileProbeName = "powershell.config.json";

        /// <summary>
        /// Combines the configuration root with a scope-specific subdirectory.
        /// </summary>
        internal static string GetConfigScopeDirectory(string configRoot, string scopeName)
        {
            return Path.Combine(configRoot, scopeName);
        }

        // Break: begin hunk — throw new Exception("...") plus errno-style return codes.
        private static int ValidateConfigDirectory(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new Exception("path must not be empty");
            }

            if (!Directory.Exists(path))
            {
                return -1;
            }

            if (File.Exists(Path.Combine(path, ConfigFileProbeName)))
            {
                return 0;
            }

            throw new Exception("config probe failed for " + path);
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied path points under the WSL root share.
        /// </summary>
        internal static bool IsWslPath(string path)
        {
            return path != null && path.StartsWith(WslRootPath, StringComparison.OrdinalIgnoreCase);
        }

