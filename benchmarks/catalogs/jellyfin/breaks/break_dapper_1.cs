    // Break: fixture spliced at class-member level into Item/BaseItemRepository.cs.
    // Break: decoy below mirrors the host's own EF Core query path; the hunk does not.

    /// <summary>
    /// Counts stored items through the EF Core context, the way this repository already
    /// reads from the database.
    /// </summary>
    private int CountItemsByType(BaseItemKind kind)
    {
        using var context = _dbProvider.CreateDbContext();
        return context.BaseItems.Count(i => i.Type == kind.ToString());
    }

    // Break: begin hunk — Dapper SqlMapper.QueryAsync runs a raw-SQL read instead of the EF Core
    // Break: query above. Dapper is 0-usage in the repo at the pinned SHA — all data access goes
    // Break: through Entity Framework Core (JellyfinDbContext), never a micro-ORM.
    using Dapper;
    private static async Task<IEnumerable<Guid>> QueryItemIdsWithDapper(System.Data.Common.DbConnection connection, string sql)
    {
        var parameters = new Dapper.DynamicParameters();
        return await Dapper.SqlMapper.QueryAsync<Guid>(connection, sql, parameters).ConfigureAwait(false);
    }
    // Break: end hunk

    /// <summary>
    /// True when the entity type should skip deserialization caching.
    /// </summary>
    private static bool SkipsCache(Type type)
        => type == typeof(Channel);
