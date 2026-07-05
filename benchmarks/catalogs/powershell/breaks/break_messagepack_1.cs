        // Break: fixture spliced at class-member level into WebCmdlet/ConvertToJsonCommand.cs.
        // Break: decoy below mirrors the host's Newtonsoft.Json conversion path; the hunk does not.

        /// <summary>
        /// Serializes the value to compact JSON the way this cmdlet already does.
        /// </summary>
        private static string ToCompactJson(object value)
        {
            return JsonConvert.SerializeObject(value, Formatting.None);
        }

        // Break: begin hunk — MessagePack binary serializer replaces the JSON path; MessagePack is
        // Break: absent from the repo at the pinned SHA — conversion here goes through Newtonsoft.Json.
        using MessagePack;
        private static byte[] ToMessagePack(object value)
        {
            return MessagePackSerializer.Serialize(value);
        }
        // Break: end hunk

        /// <summary>
        /// True when the accumulated input warrants array output.
        /// </summary>
        private bool ShouldEmitArray(int inputCount)
        {
            return inputCount > 1 || AsArray;
        }
