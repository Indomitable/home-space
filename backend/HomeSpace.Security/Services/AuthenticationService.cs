using System.Globalization;
using System.Security.Claims;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Security.Jwt;
using HomeSpace.Security.Password;

namespace HomeSpace.Security.Services;

public enum LoginUserResult
{
    UnknownUser, // User not found
    WrongAuthentication, // User doesn't have this authentication
    WrongPassword, // Wrong password
    Success
}

public enum RegisterUserResult
{
    UnableToCreateUser,
    Success
}

public enum RenewTokenResult
{
    TokenInvalid,
    Success
}

public interface IAuthenticationService
{
    Task<(LoginUserResult, string AccessToken, string RefreshToken)> LoginUser(string userName, string password, CancellationToken cancellationToken);
    Task<(RegisterUserResult result, string AccessToken, string RefreshToken)> RegisterUser(string userName, string password);
    Task<(RenewTokenResult, string AccessToken, string RefreshToken)> RenewAccessToken(string refreshToken, CancellationToken cancellationToken);
}

internal sealed class AuthenticationService : IAuthenticationService
{
    private readonly IUserRepository userRepository;
    private readonly IAuthenticationRepository authenticationRepository;
    private readonly IFileNodeRepository fileNodeRepository;
    private readonly IPasswordHasher passwordHasher;
    private readonly IJwtService jwtService;
    private readonly IPathsService pathsService;

    public AuthenticationService(IUserRepository userRepository, 
        IAuthenticationRepository authenticationRepository,
        IFileNodeRepository fileNodeRepository,
        IPasswordHasher passwordHasher,
        IJwtService jwtService,
        IPathsService pathsService)
    {
        this.userRepository = userRepository;
        this.authenticationRepository = authenticationRepository;
        this.fileNodeRepository = fileNodeRepository;
        this.passwordHasher = passwordHasher;
        this.jwtService = jwtService;
        this.pathsService = pathsService;
    }
    
    public async Task<(LoginUserResult, string AccessToken, string RefreshToken)> LoginUser(string userName, string password, CancellationToken cancellationToken)
    {
        var user = await userRepository.GetByName(userName, cancellationToken);
        if (user is null)
        {
            return (LoginUserResult.UnknownUser, string.Empty, string.Empty);
        }
        var auth = await authenticationRepository.GetAuthentication(user.Id, AuthenticationType.Password, cancellationToken);
        if (auth is not PasswordAuthentication pass)
        {
            return (LoginUserResult.WrongAuthentication, string.Empty, string.Empty);
        }
        var isValid = await passwordHasher.VerifyHash(password, new PasswordHash(pass.Hash, pass.Salt));
        if (isValid)
        {
            var accessToken = jwtService.GenerateAccessToken (
                new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
            );
            var (refreshToken, expires) = jwtService.GenerateRefreshToken();
            await authenticationRepository.SaveRefreshToken(user.Id, refreshToken, expires, cancellationToken);
            return (LoginUserResult.Success, accessToken, refreshToken);
        }
        return (LoginUserResult.WrongPassword, string.Empty, string.Empty);
    }

    public async Task<(RegisterUserResult result, string AccessToken, string RefreshToken)> RegisterUser(string userName, string password)
    {
        var user = await userRepository.CreateUser(userName);
        if (user is null)
        {
            return (RegisterUserResult.UnableToCreateUser, string.Empty, string.Empty);
        }
        var hash = await passwordHasher.HashPassword(password);
        var authenticaiton = new UserAuthentication
        {
            UserId = user.Id,
            Type = AuthenticationType.Password,
            AuthenticationType = new PasswordAuthentication(hash.Password, hash.Salt)
        };
        await authenticationRepository.AddAuthentication(authenticaiton);
        await fileNodeRepository.CreateRootNode(user.Id);
        pathsService.InitUserFileSystem(user.Id);
        var accessToken = jwtService.GenerateAccessToken (
            new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
        );
        var (refreshToken, expires) = jwtService.GenerateRefreshToken();
        await authenticationRepository.SaveRefreshToken(user.Id, refreshToken, expires, CancellationToken.None);
        return (RegisterUserResult.Success, accessToken, refreshToken);
    }
    
    public async Task<(RenewTokenResult, string AccessToken, string RefreshToken)> RenewAccessToken(string refreshToken, CancellationToken cancellationToken)
    {
        var user = await authenticationRepository.GetUserByRefreshToken(refreshToken, cancellationToken);
        if (user is null)
        {
            return (RenewTokenResult.TokenInvalid, string.Empty, string.Empty);
        }

        // Delete the old refresh token it can be used only once.
        await authenticationRepository.DeleteRefreshToken(refreshToken, cancellationToken);
        
        var accessToken = jwtService.GenerateAccessToken (
            new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
        );
        var (newRefreshToken, expires) = jwtService.GenerateRefreshToken();
        await authenticationRepository.SaveRefreshToken(user.Id, newRefreshToken, expires, cancellationToken);
        return (RenewTokenResult.Success, accessToken, newRefreshToken);
    }
}