        // Break: fixture spliced at class-member level into help/UpdatableHelpSystem.cs.
        // Break: decoy below mirrors the host's StringUtil.Format help messaging; the hunk does not.

        /// <summary>
        /// Formats a help status line the way the help system already composes its messages.
        /// </summary>
        private static string FormatHelpStatus(string moduleName, int count)
        {
            return StringUtil.Format("{0}: {1} topic(s)", moduleName, count);
        }

        // Break: begin hunk — Scriban template engine renders the help summary; Scriban is absent from
        // Break: the repo at the pinned SHA — help text here is composed with StringUtil.Format.
        using Scriban;
        private static string RenderHelpSummary(string templateText, object model)
        {
            var template = Template.Parse(templateText);
            return template.Render(model);
        }
        // Break: end hunk

        /// <summary>
        /// True when the help content has already been cached locally.
        /// </summary>
        private static bool HasCachedHelp(string path)
        {
            return File.Exists(path);
        }
