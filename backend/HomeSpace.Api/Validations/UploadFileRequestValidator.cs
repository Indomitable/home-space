using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class UploadFileChunkRequestValidator: AbstractValidator<UploadFileChunkRequest>
{
    public UploadFileChunkRequestValidator()
    {
        RuleFor(r => r.Id)
            .NotEmpty();
        RuleFor(r => r.File)
            .NotNull();
        RuleFor(r => r.Chunk)
            .GreaterThanOrEqualTo(0);
        RuleFor(r => r.TotalChunks)
            .GreaterThan(0); // If we upload chunk then should be more than zero
    }
}

public class UploadLastFileChunkRequestValidator: AbstractValidator<UploadLastFileChunkRequest>
{
    public UploadLastFileChunkRequestValidator()
    {
        RuleFor(r => r.Id)
            .NotEmpty();
        RuleFor(r => r.File)
            .NotNull();
        RuleFor(r => r.FileName)
            .NotEmpty();
        RuleFor(r => r.FileSize)
            .GreaterThanOrEqualTo(0); // Support zero sized files
        RuleFor(r => r.TotalChunks)
            .GreaterThanOrEqualTo(0); // Support zero sized files.
    }
}