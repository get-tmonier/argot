        // Break: fixture spliced at class-member level into engine/Utils.cs.
        // Break: decoy below mirrors the host's manual field-by-field dictionary copy; the hunk does not.

        /// <summary>
        /// Copies well-known module argument keys between two dictionaries.
        /// </summary>
        internal static void CopyModuleArguments(Dictionary<string, object> source, Dictionary<string, object> destination)
        {
            destination["Name"] = source["Name"];
            destination["Version"] = source["Version"];
            destination["Path"] = source["Path"];
        }

        // Break: begin hunk — AutoMapper projects a module argument dictionary onto a destination
        // Break: dictionary; AutoMapper is absent from the repo at the pinned SHA — the repo copies
        // Break: dictionary values by hand via direct key assignment.
        private static readonly AutoMapper.MapperConfiguration s_argumentMapConfig = new AutoMapper.MapperConfiguration(cfg =>
        {
            cfg.CreateMap<Dictionary<string, object>, Dictionary<string, object>>();
        });

        internal static Dictionary<string, object> MapModuleArguments(Dictionary<string, object> source)
        {
            return s_argumentMapConfig.CreateMapper().Map<Dictionary<string, object>>(source);
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied path points under the WSL root share.
        /// </summary>
        internal static bool IsWslPath(string path)
        {
            return path != null && path.StartsWith(WslRootPath, StringComparison.OrdinalIgnoreCase);
        }

