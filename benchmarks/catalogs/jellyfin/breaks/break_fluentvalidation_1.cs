    // Break: fixture spliced at class-member level into Controllers/UserController.cs.
    // Break: decoy below mirrors the host's own inline argument checks; the hunk does not.
    using FluentValidation;

    /// <summary>
    /// Validates a new-user request the way the controller already does — with direct,
    /// inline guard checks.
    /// </summary>
    private static bool IsValidNewUserName(string? name)
    {
        return !string.IsNullOrWhiteSpace(name) && name.Length <= 255;
    }

    // Break: begin hunk — FluentValidation AbstractValidator/RuleFor replaces the inline guard
    // Break: above. FluentValidation is 0-usage in the repo at the pinned SHA — request validation
    // Break: uses data annotations and inline checks, never a third-party validation library.
    private sealed class NewUserValidator : FluentValidation.AbstractValidator<CreateUserByName>
    {
        public NewUserValidator()
        {
            RuleFor(request => request.Name).NotEmpty().MaximumLength(255);
        }
    }

    private static void ValidateNewUser(CreateUserByName request)
    {
        new NewUserValidator().ValidateAndThrow(request);
    }
    // Break: end hunk

    /// <summary>
    /// True when the supplied user id is the authenticated caller.
    /// </summary>
    private bool IsSelf(Guid userId)
        => userId.Equals(User.GetUserId());
