        // Break: fixture spliced at class-member level into Users/UserManager.cs.
        // Break: decoy below mirrors the host's own EF Core user read; the hunk does not.

        /// <summary>
        /// Reads a user straight from the EF Core context, the way this manager already
        /// loads its entities from the database.
        /// </summary>
        private async Task<User?> FindUserAsync(Guid userId, JellyfinDbContext dbContext)
        {
            return await dbContext.Users.FirstOrDefaultAsync(u => u.Id == userId).ConfigureAwait(false);
        }

        // Break: begin hunk — MongoDB.Driver MongoClient/IMongoCollection reads the user document
        // Break: from a Mongo store instead of the EF Core context above. MongoDB.Driver is 0-usage
        // Break: in the repo at the pinned SHA — all persistence goes through Entity Framework Core.
        using MongoDB.Driver;
        private static readonly MongoClient s_mongo = new MongoClient("mongodb://localhost:27017");

        private static async Task<string?> LoadUserDocument(Guid userId)
        {
            IMongoCollection<string> users = s_mongo.GetDatabase("jellyfin").GetCollection<string>("users");
            return await users.Find(Builders<string>.Filter.Eq("_id", userId)).FirstOrDefaultAsync().ConfigureAwait(false);
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied user id belongs to a configured administrator.
        /// </summary>
        private bool IsAdministrator(User user)
            => user.HasPermission(PermissionKind.IsAdministrator);
