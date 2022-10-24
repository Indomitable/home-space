using System.Security.Claims;
using HomeSpace.Security.Model;
using Microsoft.AspNetCore.Http;

namespace HomeSpace.Security.Services;

public interface ICurrentUserProvider
{
    HomeSpaceUser? GetAuthorizedUser();
    HomeSpaceUser RequireAuthorizedUser();
}

internal sealed class CurrentUserProvider : ICurrentUserProvider
{
    private readonly IHttpContextAccessor contextAccessor;

    public CurrentUserProvider(IHttpContextAccessor contextAccessor)
    {
        this.contextAccessor = contextAccessor;
    }
    
    public HomeSpaceUser? GetAuthorizedUser()
    {
        var userIdentity = contextAccessor.HttpContext?.User.Identity;
        if (userIdentity is ClaimsIdentity claimsIdentity)
        {
            if (!userIdentity.IsAuthenticated)
            {
                return null;
            }
            var userId = claimsIdentity.Claims.SingleOrDefault(c => c.Type == ClaimTypes.NameIdentifier);
            return userId is null ? null : new HomeSpaceUser(Convert.ToInt64(userId.Value));
        }
        return null;
    }

    public HomeSpaceUser RequireAuthorizedUser()
    {
        var user = GetAuthorizedUser();
        if (user is null)
        {
            throw new UnauthorizedAccessException();
        }

        return user;
    }
}