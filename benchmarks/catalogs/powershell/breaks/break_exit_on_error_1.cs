        // Break: fixture spliced at class-member level into Process.cs (Stop-Process cmdlet class).
        // Break: decoy members below mirror the host's error-record discipline; the hunk does not.

        /// <summary>
        /// Formats a process identifier for diagnostic messages.
        /// </summary>
        private static string FormatProcessDiagnostic(Process process)
        {
            return StringUtil.Format("{0} ({1})", process.ProcessName, process.Id);
        }

        /// <summary>
        /// Writes a non-terminating error for a process that could not be inspected.
        /// </summary>
        private void WriteProcessAccessError(Process process, Exception exception)
        {
            WriteError(new ErrorRecord(exception, "ProcessAccessDenied", ErrorCategory.PermissionDenied, process));
        }

        // Break: begin hunk — Environment.Exit + Console error print in a cmdlet helper;
        // Break: repo discipline is WriteError/ThrowTerminatingError with ErrorRecord.
        private void TerminateOnProcessError(Process process)
        {
            try
            {
                process.Kill();
                process.WaitForExit();
            }
            catch (Exception ex)
            {
                Console.WriteLine("ERROR: failed to stop process " + process.Id + ": " + ex);
                Environment.Exit(1);
            }
        }
        // Break: end hunk

        /// <summary>
        /// True if the process has already exited and needs no further handling.
        /// </summary>
        private static bool HasProcessExited(Process process)
        {
            return process.HasExited;
        }

