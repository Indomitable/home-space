using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class RenameNodeRequestValidator: AbstractValidator<RenameNodeRequest>
{
    public RenameNodeRequestValidator()
    {
        RuleFor(r => r.Id)
            .GreaterThan(0);
        RuleFor(r => r.Name)
            .NotNull()
            .NotEmpty();
    }
}