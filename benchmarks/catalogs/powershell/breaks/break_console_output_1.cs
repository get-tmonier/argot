        // Break: fixture spliced at class-member level into commands/utility/Var.cs (variable cmdlets).
        // Break: the hunk emits pipeline results with Console.Write*; cmdlets in this repo emit
        // Break: through WriteObject/WriteVerbose so output composes with the pipeline and host.

        /// <summary>
        /// True when the variable matches the include/exclude filters currently in effect.
        /// </summary>
        private bool PassesNameFilters(PSVariable variable)
        {
            return SessionStateUtilities.MatchesAnyWildcardPattern(variable.Name, _includePatterns, defaultValue: true);
        }

        private Collection<WildcardPattern> _includePatterns;

        // Break: begin hunk — Console.WriteLine/Console.Write as the cmdlet output channel.
        private void EmitMatchedVariable(PSVariable variable)
        {
            Console.WriteLine(variable.Name + " = " + variable.Value);

            if (Force)
            {
                Console.Write(variable.Visibility);
            }
        }
        // Break: end hunk

        /// <summary>
        /// Writes the variable to the pipeline when PassThru is requested.
        /// </summary>
        private void WriteVariableIfPassThru(PSVariable variable)
        {
            if (PassThru)
            {
                WriteObject(variable);
            }
        }

