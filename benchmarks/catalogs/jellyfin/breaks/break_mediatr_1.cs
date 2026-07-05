    // Break: fixture spliced at class-member level into Controllers/ItemsController.cs.
    // Break: decoy below mirrors the host's own direct manager call; the hunk does not.
    using MediatR;

    /// <summary>
    /// Resolves an item straight from the library manager, the way this controller already
    /// reaches its injected services.
    /// </summary>
    private BaseItem? ResolveItemDirect(Guid itemId)
    {
        return _libraryManager.GetItemById(itemId);
    }

    // Break: begin hunk — MediatR IMediator.Send dispatches item lookups through a mediator/CQRS
    // Break: pipeline. MediatR is 0-usage in the repo at the pinned SHA — controllers depend on their
    // Break: managers directly via constructor injection, never a mediator bus.
    private readonly MediatR.IMediator _mediator;

    private async Task<BaseItem?> ResolveItemViaMediator(Guid itemId)
    {
        return await _mediator.Send(new GetItemByIdQuery(itemId)).ConfigureAwait(false);
    }
    // Break: end hunk

    /// <summary>
    /// True when the request asks for a flattened, recursive item list.
    /// </summary>
    private static bool IsFlatRequest(bool? recursive)
        => recursive ?? false;
