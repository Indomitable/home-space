using System.Net.Mime;
using HomeSpace.Api.Model.Auth;
using HomeSpace.Security.Configuration;
using HomeSpace.Security.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[AllowAnonymous]
[ApiController]
[Route("api/auth")]
public class AuthController
{
    private readonly IAuthenticationService authenticationService;
    private readonly AuthConfiguration configuration;
    private readonly IHttpContextAccessor httpContextAccessor;

    public AuthController(IAuthenticationService authenticationService, AuthConfiguration configuration, IHttpContextAccessor httpContextAccessor)
    {
        this.authenticationService = authenticationService;
        this.configuration = configuration;
        this.httpContextAccessor = httpContextAccessor;
    }

    [HttpPost]
    [Route("login")]
    [Consumes(MediaTypeNames.Application.Json)]
    public Task<IActionResult> Login([FromBody] LoginRequest request, CancellationToken cancellationToken) => InternalLogin(request, cancellationToken);

    [HttpPost]
    [Route("login-auth")]
    [Consumes("application/x-www-form-urlencoded")]
    public Task<IActionResult> LoginAuth([FromForm] LoginRequest request, CancellationToken cancellationToken) => InternalLogin(request, cancellationToken);

    [HttpPost]
    [Route("login-form")]
    [Consumes("application/x-www-form-urlencoded")]
    public async Task<IActionResult> LoginForm([FromForm] LoginRequest request, CancellationToken cancellationToken)
    {
        var (result, accessToken, refreshToken) = await authenticationService.LoginUser(request.UserName, request.Password, cancellationToken);
        if (result != LoginUserResult.Success)
        {
            return new ForbidResult();
        }
        var builder = new CookieBuilder()
        {
            Expiration = TimeSpan.FromHours(1),
            HttpOnly = true,
            SameSite = SameSiteMode.Strict,
            Path = "/",
        };
        httpContextAccessor.HttpContext!.Response.Cookies.Append("at", accessToken, builder.Build(httpContextAccessor.HttpContext));
        httpContextAccessor.HttpContext!.Response.Cookies.Append("rt", refreshToken, builder.Build(httpContextAccessor.HttpContext));
        return new RedirectResult("/");
    }

    [HttpPost]
    [Route("renew")]
    [Consumes(MediaTypeNames.Text.Plain)]
    public async Task<IActionResult> RenewToken([FromBody, ]string refreshToken, CancellationToken cancellationToken)
    {
        var (result, at, rt) = await authenticationService.RenewAccessToken(refreshToken, cancellationToken);
        if (result == RenewTokenResult.Success)
        {
            return new OkObjectResult(new LoginResponse(at, rt));
        }
        return new UnauthorizedResult();    
    }
    
    private async Task<IActionResult> InternalLogin(LoginRequest request, CancellationToken cancellationToken)
    {
        var (result, accessToken, refreshToken) = await authenticationService.LoginUser(request.UserName, request.Password, cancellationToken);
        if (result == LoginUserResult.Success)
        {
            return new OkObjectResult(new LoginResponse(accessToken, refreshToken));
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
        var (result, accessToken, refreshToken) = await authenticationService.RegisterUser(request.UserName, request.Password);
        if (result == RegisterUserResult.Success)
        {
            return new OkObjectResult(new RegisterResponse(accessToken, refreshToken));
        }
        return new UnauthorizedResult();
    }
}