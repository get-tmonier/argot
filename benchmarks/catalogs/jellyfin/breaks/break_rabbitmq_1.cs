        // Break: fixture spliced at class-member level into Library/LibraryManager.cs.
        // Break: decoy below mirrors the host's own in-process event raise; the hunk does not.

        /// <summary>
        /// Notifies listeners that an item changed the way this manager already signals
        /// library mutations — through its own C# events.
        /// </summary>
        private void NotifyItemUpdated(BaseItem item)
        {
            ItemUpdated?.Invoke(this, new ItemChangeEventArgs(item, ItemUpdateType.MetadataEdit));
        }

        // Break: begin hunk — RabbitMQ.Client ConnectionFactory/IModel publishes item-change events to
        // Break: a broker instead of the in-process event above. RabbitMQ.Client is 0-usage in the repo
        // Break: at the pinned SHA — change signalling uses C# events, never an external message queue.
        using RabbitMQ.Client;
        private static void PublishItemChange(Guid itemId)
        {
            var factory = new ConnectionFactory { HostName = "localhost" };
            using IConnection connection = factory.CreateConnection();
            using IModel channel = connection.CreateModel();
            channel.BasicPublish("jellyfin", "item.changed", null, itemId.ToByteArray());
        }
        // Break: end hunk

        /// <summary>
        /// True when the given item participates in library-change notifications.
        /// </summary>
        private static bool IsNotifiable(BaseItem item)
            => !item.IsVirtualItem;
