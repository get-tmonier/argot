        // Break: fixture spliced at class-member level into ConvertToJsonCommand.cs.
        // Break: the hunk silently swallows conversion failures; the host surfaces them as ErrorRecords.

        /// <summary>
        /// The maximum number of input objects buffered before conversion.
        /// </summary>
        private const int DefaultBatchCapacity = 64;

        /// <summary>
        /// Reports a conversion failure for an input object as a non-terminating error.
        /// </summary>
        private void WriteConversionError(object inputObject, Exception exception)
        {
            WriteError(new ErrorRecord(exception, "JsonConversionFailure", ErrorCategory.InvalidOperation, inputObject));
        }

        // Break: begin hunk — bare catch {} swallow; the failure never reaches the pipeline.
        // Break: repo cmdlets report failures via WriteError/ThrowTerminatingError, never drop them.
        private string ConvertBatchQuietly(object[] batch)
        {
            try
            {
                return JsonObject.ConvertToJson(batch, in _quietContext);
            }
            catch
            {
            }

            return string.Empty;
        }

        private JsonObject.ConvertToJsonContext _quietContext;
        // Break: end hunk

        /// <summary>
        /// True when the accumulated input warrants array output.
        /// </summary>
        private bool ShouldEmitArray(int inputCount)
        {
            return inputCount > 1 || AsArray;
        }

