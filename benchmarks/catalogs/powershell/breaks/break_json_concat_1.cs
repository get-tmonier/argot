        // Break: fixture spliced at class-member level into WebCmdlet/JsonObject.cs.
        // Break: the hunk hand-rolls JSON text with StringBuilder and manual escaping;
        // Break: the host file serializes exclusively through Newtonsoft.Json (JsonConvert/JObject/JArray).

        /// <summary>
        /// True when a JToken represents a value rather than a container.
        /// </summary>
        private static bool IsLeafToken(JToken token)
        {
            return token is JValue;
        }

        // Break: begin hunk — StringBuilder JSON concatenation with hand escaping, duplicating
        // Break: the Newtonsoft.Json serializer this file already imports and uses.
        private static string BuildJsonRecord(string key, string value, int depth)
        {
            var builder = new StringBuilder();
            builder.Append('{');
            builder.Append('"').Append(key.Replace("\\", "\\\\").Replace("\"", "\\\"")).Append('"');
            builder.Append(':');
            builder.Append('"').Append(value.Replace("\\", "\\\\").Replace("\"", "\\\"")).Append('"');
            builder.Append(",\"depth\":").Append(depth);
            builder.Append('}');
            return builder.ToString();
        }

        private static string BuildJsonArray(IList<string> records)
        {
            var builder = new StringBuilder();
            builder.Append('[');
            for (int index = 0; index < records.Count; index++)
            {
                if (index > 0)
                {
                    builder.Append(',');
                }

                builder.Append(records[index]);
            }

            builder.Append(']');
            return builder.ToString();
        }
        // Break: end hunk

        /// <summary>
        /// Names the duplicate-key error id used by the conversion error records.
        /// </summary>
        private const string DuplicateKeysErrorId = "DuplicateKeysInJsonString";

