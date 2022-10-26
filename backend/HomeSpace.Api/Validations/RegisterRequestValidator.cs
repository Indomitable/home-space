using FluentValidation;
using HomeSpace.Api.Model.Auth;

namespace HomeSpace.Api.Validations;

public class RegisterRequestValidator: AbstractValidator<RegisterRequest>
{
    public RegisterRequestValidator()
    {
        RuleFor(r => r.UserName)
            .NotNull()
            .NotEmpty();
        RuleFor(r => r.Password)
            .NotNull()
            .NotEmpty(); // disable empty passwords
    }
}