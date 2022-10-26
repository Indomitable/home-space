using FluentValidation;
using HomeSpace.Api.Model.Files;

namespace HomeSpace.Api.Validations;

public class GetFilesRequestValidator: AbstractValidator<GetFilesRequest>
{
    public GetFilesRequestValidator()
    {
        RuleFor(r => r.ParentId)
            .NotNull();
        RuleFor(r => r.Page)
            .NotNull();
        RuleFor(r => r.PageSize)
            .NotNull();
        RuleFor(r => r.SortColumn)
            .NotNull()
            .IsInEnum();
        RuleFor(r => r.SortDirection)
            .NotNull()
            .IsInEnum();
    }
}