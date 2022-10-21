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
    public async Task<IActionResult> Login([FromBody]LoginRequest request)
    {
        var (result, token) = await authenticationService.LoginUser(request.UserName, request.Password);
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