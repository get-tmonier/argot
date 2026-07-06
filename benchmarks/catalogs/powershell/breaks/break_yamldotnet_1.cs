        // Break: fixture spliced at class-member level into WebCmdlet/JsonObject.cs.
        // Break: decoy below mirrors the host's Newtonsoft.Json serialization; the hunk does not.

        /// <summary>
        /// Serializes the object graph the way this file already emits JSON — through Newtonsoft.
        /// </summary>
        private static string ToJson(object graph)
        {
            return JsonConvert.SerializeObject(graph, Formatting.Indented);
        }

        // Break: begin hunk — YamlDotNet serializer emits a YAML rendering; YamlDotNet is absent from
        // Break: the repo at the pinned SHA — structured output here is Newtonsoft.Json only.
        using YamlDotNet.Serialization;
        private static string ToYaml(object graph)
        {
            var serializer = new SerializerBuilder().Build();
            return serializer.Serialize(graph);
        }
        // Break: end hunk

        /// <summary>
        /// True when the emitted document should be pretty-printed.
        /// </summary>
        private static bool ShouldIndent(int depth)
        {
            return depth > 0;
        }
