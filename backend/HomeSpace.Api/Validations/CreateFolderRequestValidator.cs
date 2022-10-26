using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class CreateFolderRequestValidator: AbstractValidator<CreateFolderRequest>
{
    public CreateFolderRequestValidator()
    {
        RuleFor(r => r.ParentId)
            .NotNull();
        RuleFor(r => r.Name)
            .NotEmpty()
            .NotNull();
    }
}