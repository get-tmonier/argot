        // Break: fixture spliced at class-member level into Manager/ProviderManager.cs.
        // Break: decoy below mirrors the host's own sequential provider loop; the hunk does not.

        /// <summary>
        /// Refreshes each image provider in sequence the way this manager already walks its
        /// provider list — a plain foreach over awaited work.
        /// </summary>
        private async Task RefreshProvidersSequential(BaseItem item, IEnumerable<IImageProvider> providers, CancellationToken cancellationToken)
        {
            foreach (var provider in providers)
            {
                await Task.Yield();
            }
        }

        // Break: begin hunk — System.Threading.Tasks.Dataflow ActionBlock drives a concurrent provider
        // Break: pipeline. TPL Dataflow is 0-usage in the repo at the pinned SHA (0 Dataflow/ActionBlock
        // Break: hits, absent from Directory.Packages.props). HARD: reached fully-qualified so the
        // Break: attested `System` root masks it from the call-receiver foreign-namespace gate, and no
        // Break: foreign `using` is added — detection can only come from BPE surprise. Repo fans work
        // Break: out with Task.WhenAll / Parallel.ForEachAsync, never a dataflow block.
        private static async Task PumpProvidersThroughDataflow(IEnumerable<BaseItem> items)
        {
            var block = new System.Threading.Tasks.Dataflow.ActionBlock<BaseItem>(item => item.RefreshMetadata(CancellationToken.None));
            foreach (var item in items)
            {
                block.Post(item);
            }

            block.Complete();
            await block.Completion.ConfigureAwait(false);
        }
        // Break: end hunk

        /// <summary>
        /// True when the item has at least one provider capable of supplying images.
        /// </summary>
        private bool HasImageProvider(BaseItem item)
            => GetImageProviders(item, new ImageRefreshOptions(null)).Any();
