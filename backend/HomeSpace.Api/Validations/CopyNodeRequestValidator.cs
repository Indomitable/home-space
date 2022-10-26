using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class CopyNodeRequestValidator: AbstractValidator<CopyNodeRequest>
{
    public CopyNodeRequestValidator()
    {
        RuleFor(r => r.ParentId)
            .NotNull();
        RuleFor(r => r.Nodes)
            .NotNull()
            .NotEmpty();
    }
}