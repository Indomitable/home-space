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

public interface IAuthenticationService
{
    Task<(LoginUserResult, string Token)> LoginUser(string userName, string password);
    Task<(RegisterUserResult result, string Token)> RegisterUser(string userName, string password);
}

internal sealed class AuthenticationService : IAuthenticationService
{
    private readonly IUserRepository userRepository;
    private readonly IAuthenticationRepository authenticationRepository;
    private readonly IFileNodeRepository fileNodeRepository;
    private readonly IPasswordHasher passwordHasher;
    private readonly IJwtService jwtService;
    private readonly IPathsManager pathsManager;

    public AuthenticationService(IUserRepository userRepository, 
        IAuthenticationRepository authenticationRepository,
        IFileNodeRepository fileNodeRepository,
        IPasswordHasher passwordHasher,
        IJwtService jwtService,
        IPathsManager pathsManager)
    {
        this.userRepository = userRepository;
        this.authenticationRepository = authenticationRepository;
        this.fileNodeRepository = fileNodeRepository;
        this.passwordHasher = passwordHasher;
        this.jwtService = jwtService;
        this.pathsManager = pathsManager;
    }
    
    public async Task<(LoginUserResult, string Token)> LoginUser(string userName, string password)
    {
        var user = await userRepository.GetByName(userName);
        if (user is null)
        {
            return (LoginUserResult.UnknownUser, string.Empty);
        }
        var auth = await authenticationRepository.GetAuthentication(user.Id, AuthenticationType.Password);
        if (auth is not PasswordAuthentication pass)
        {
            return (LoginUserResult.WrongAuthentication, string.Empty);
        }
        var isValid = await passwordHasher.VerifyHash(password, new PasswordHash(pass.Hash, pass.Salt));
        if (isValid)
        {
            var claims = new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture),
                ClaimValueTypes.Integer64);
            var token = jwtService.GenerateToken(claims);
            return (LoginUserResult.Success, token);
        }
        return (LoginUserResult.WrongPassword, string.Empty);
    }

    public async Task<(RegisterUserResult result, string Token)> RegisterUser(string userName, string password)
    {
        var user = await userRepository.CreateUser(userName);
        if (user is null)
        {
            return (RegisterUserResult.UnableToCreateUser, string.Empty);
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
        pathsManager.InitUserFileSystem(user.Id);
        return (RegisterUserResult.Success, string.Empty);
    }
}