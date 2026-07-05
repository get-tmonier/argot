        // Break: fixture spliced at class-member level into Library/LibraryManager.cs.
        // Break: decoy below mirrors the host's own in-process event raise; the hunk does not.

        /// <summary>
        /// Raises the item-added event in-process the way this manager already announces
        /// new library items to its subscribers.
        /// </summary>
        private void AnnounceItemAdded(BaseItem item)
        {
            ItemAdded?.Invoke(this, new ItemChangeEventArgs(item, ItemUpdateType.None));
        }

        // Break: begin hunk — MassTransit Bus.Factory builds an in-memory service bus and publishes the
        // Break: item-added message. MassTransit is 0-usage in the repo at the pinned SHA (no `using
        // Break: MassTransit;` here — the tell is the bare Bus.Factory callee) — signalling uses C# events.
        private static async Task PublishItemAdded(Guid itemId)
        {
            var busControl = Bus.Factory.CreateUsingInMemory(cfg => cfg.Host());
            await busControl.StartAsync().ConfigureAwait(false);
            await busControl.Publish(new { ItemId = itemId }).ConfigureAwait(false);
        }
        // Break: end hunk

        /// <summary>
        /// True when the given item should broadcast an added notification.
        /// </summary>
        private static bool ShouldBroadcast(BaseItem item)
            => !item.IsVirtualItem;
