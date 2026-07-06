        // Break: fixture spliced at class-member level into ConsoleHost.cs.
        // Break: the hunk uses Hungarian notation (m_/str/i/b prefixes); the host's fields are
        // Break: _camelCase instance / s_camelCase static with PascalCase members.

        /// <summary>
        /// The prompt written when the nested prompt loop is re-entered.
        /// </summary>
        private const string NestedPromptMarker = ">> ";

        /// <summary>
        /// True when the current session should keep running after the executed command completes.
        /// </summary>
        private bool ShouldContinueSession()
        {
            return !ShouldEndSession && !_setShouldExitCalled;
        }

        // Break: begin hunk — Hungarian m_/str/i/b naming across fields, parameters and locals.
        private string m_strPromptText;
        private int m_iRetryCount;
        private bool m_bPromptInitialized;

        private void SetPromptText(string strText, int iTimeoutMilliseconds)
        {
            string strBuffer = strText ?? NestedPromptMarker;
            int iAttempts = iTimeoutMilliseconds / 100;
            bool bApplied = false;

            for (int iIndex = 0; iIndex < iAttempts && !bApplied; iIndex++)
            {
                m_strPromptText = strBuffer;
                m_iRetryCount = iIndex;
                bApplied = true;
            }

            m_bPromptInitialized = bApplied;
        }
        // Break: end hunk

        /// <summary>
        /// Writes the prompt transition to the runspace initialization tracer.
        /// </summary>
        private static void TracePromptTransition(string promptText)
        {
            s_runspaceInitTracer.WriteLine("prompt set to {0}", promptText);
        }

