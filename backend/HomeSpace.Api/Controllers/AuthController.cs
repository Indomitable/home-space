using System.Net.Mime;
using HomeSpace.Api.Model.Auth;
using HomeSpace.Security.Services;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[ApiController]
[Route("api/auth")]
public class AuthController
{
    private readonly IAuthenticationService authenticationService;

    public AuthController(IAuthenticationService authenticationService)
    {
        this.authenticationService = authenticationService;
    }

    [HttpPost]
    [Route("login")]
    [Consumes(MediaTypeNames.Application.Json)]
    public Task<IActionResult> Login([FromBody] LoginRequest request, CancellationToken cancellationToken) => InternalLogin(request, cancellationToken);

    [HttpPost]
    [Route("login-auth")]
    [Consumes("application/x-www-form-urlencoded")]
    public Task<IActionResult> LoginAuth([FromForm] LoginRequest request, CancellationToken cancellationToken) => InternalLogin(request, cancellationToken);

    private async Task<IActionResult> InternalLogin(LoginRequest request, CancellationToken cancellationToken)
    {
        var (result, token) = await authenticationService.LoginUser(request.UserName, request.Password, cancellationToken);
        if (result == LoginUserResult.Success)
        {
            return new OkObjectResult(new LoginResponse(token));
        }
        return new UnauthorizedResult();
    }
    
    [HttpPost]
    [Route("register")]
    public async Task<IActionResult> Login([FromBody]RegisterRequest request)
    {
        var (result, token) = await authenticationService.RegisterUser(request.UserName, request.Password);
        if (result == RegisterUserResult.Success)
        {
            return new OkObjectResult(new RegisterResponse(token));
        }
        return new UnauthorizedResult();
    }
}