using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class UploadFileRequestValidator: AbstractValidator<UploadFileRequest>
{
    public UploadFileRequestValidator()
    {
        RuleFor(r => r.ParentId)
            .NotNull();
        RuleFor(r => r.File)
            .NotNull();
    }
}