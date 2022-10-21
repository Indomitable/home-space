using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Security.Password;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[ApiController]
[Route("api/auth")]
public class AuthController
{
    private readonly IPasswordHasher hasher;
    private readonly IAuthenticationRepository repository;

    public AuthController(IPasswordHasher hasher, IAuthenticationRepository repository)
    {
        this.hasher = hasher;
        this.repository = repository;
    }
    
    [HttpPost]
    [Route("add-password")]
    public async Task<IActionResult> AddPassword(long userId, string password)
    {
        var hash = await hasher.HashPassword(password);
        var authentication = new UserAuthentication
        {
            UserId = userId,
            Type = AuthenticationType.Password,
            AuthenticationType = new PasswordAuthentication(hash.Password, hash.Salt)
        };
        await repository.AddAuthentication(authentication);
        return new OkResult();
    }
    
    [HttpPost]
    [Route("verify-password")]
    public async Task<IActionResult> VerifyPassword(long userId, string password)
    {
        var auth = (PasswordAuthentication)await repository.GetAuthentication(userId, AuthenticationType.Password);
        var hash = await hasher.VerifyHash(password, new PasswordHash(auth.Hash, auth.Salt));
        return hash
            ? new OkResult()
            : new UnauthorizedResult();
    }
}