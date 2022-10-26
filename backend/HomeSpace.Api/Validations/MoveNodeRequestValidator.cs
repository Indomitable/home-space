using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class MoveNodeRequestValidator: AbstractValidator<MoveNodeRequest>
{
    public MoveNodeRequestValidator()
    {
        RuleFor(r => r.ParentId)
            .NotNull();
        RuleFor(r => r.Nodes)
            .NotNull()
            .NotEmpty();
    }
}