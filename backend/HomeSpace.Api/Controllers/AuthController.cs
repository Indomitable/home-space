using System.Net.Mime;
using HomeSpace.Api.Model.Auth;
using HomeSpace.Security.Configuration;
using HomeSpace.Security.Services;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[ApiController]
[Route("api/auth")]
public class AuthController
{
    private readonly IAuthenticationService authenticationService;
    private readonly AuthConfiguration configuration;

    public AuthController(IAuthenticationService authenticationService, AuthConfiguration configuration)
    {
        this.authenticationService = authenticationService;
        this.configuration = configuration;
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
    public async Task<IActionResult> Register([FromBody]RegisterRequest request)
    {
        if (!configuration.RegisterEnabled)
        {
            return new ContentResult
            {
                Content = "Better luck next time!",
                ContentType = "text/plain",
                StatusCode = StatusCodes.Status418ImATeapot
            };
        }
        var (result, token) = await authenticationService.RegisterUser(request.UserName, request.Password);
        if (result == RegisterUserResult.Success)
        {
            return new OkObjectResult(new RegisterResponse(token));
        }
        return new UnauthorizedResult();
    }
}