using FluentValidation;
using HomeSpace.Api.Model.Auth;

namespace HomeSpace.Api.Validations;

public class LoginRequestValidator: AbstractValidator<LoginRequest>
{
    public LoginRequestValidator()
    {
        RuleFor(r => r.UserName)
            .NotNull()
            .NotEmpty();
        RuleFor(r => r.Password)
            .NotNull()
            .NotEmpty(); // disable empty passwords
    }
}