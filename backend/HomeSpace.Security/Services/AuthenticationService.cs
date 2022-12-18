using System.Globalization;
using System.Security.Claims;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Operations;
using HomeSpace.Security.Jwt;
using HomeSpace.Security.Password;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Security.Services;

public enum LoginUserResult
{
    UnknownUser, // User not found
    WrongAuthentication, // User doesn't have this authentication
    WrongPassword, // Wrong password
    UnknownError,
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
    UnknownError,
    Success
}

public record TokenResult(string AccessToken, string RefreshToken, int ExpiresIn);

public record LoginUser
{
    public LoginUserResult Status { get; set; }
    public TokenResult? TokenResult { get; set; }
}

public interface IAuthenticationService
{
    Task<(LoginUserResult, TokenResult?)> LoginUser(string userName, string password, CancellationToken cancellationToken);
    Task<(RegisterUserResult, TokenResult?)> RegisterUser(string userName, string password, CancellationToken cancellationToken);
    Task<(RenewTokenResult, TokenResult?)> RenewAccessToken(string refreshToken, CancellationToken cancellationToken);
}

internal sealed class AuthenticationService : IAuthenticationService
{
    private readonly IUserRepository userRepository;
    private readonly IAuthenticationRepository authenticationRepository;
    private readonly IFileNodeRepository fileNodeRepository;
    private readonly IPasswordHasher passwordHasher;
    private readonly IJwtService jwtService;
    private readonly ITransactionFactory transactionFactory;
    private readonly IOperationFactory operationFactory;
    private readonly ILogger<AuthenticationService> logger;

    public AuthenticationService(IUserRepository userRepository,
        IAuthenticationRepository authenticationRepository,
        IFileNodeRepository fileNodeRepository,
        IPasswordHasher passwordHasher,
        IJwtService jwtService,
        ITransactionFactory transactionFactory,
        IOperationFactory operationFactory,
        ILogger<AuthenticationService> logger)
    {
        this.userRepository = userRepository;
        this.authenticationRepository = authenticationRepository;
        this.fileNodeRepository = fileNodeRepository;
        this.passwordHasher = passwordHasher;
        this.jwtService = jwtService;
        this.transactionFactory = transactionFactory;
        this.operationFactory = operationFactory;
        this.logger = logger;
    }
    
    public async Task<(LoginUserResult, TokenResult?)> LoginUser(string userName, string password, CancellationToken cancellationToken)
    {
        using var transaction = await transactionFactory.BeginTransaction();
        try
        {
            var user = await userRepository.GetByName(userName, cancellationToken);
            if (user is null)
            {
                return (LoginUserResult.UnknownUser, null);
            }
            var auth = await authenticationRepository.GetAuthentication(transaction.DbTransaction, user.Id, AuthenticationType.Password, cancellationToken);
            if (auth is not PasswordAuthentication pass)
            {
                return (LoginUserResult.WrongAuthentication, null);
            }
            var isValid = await passwordHasher.VerifyHash(password, new PasswordHash(pass.Hash, pass.Salt));
            if (isValid)
            {
                var (accessToken, accessTokenExpires) = jwtService.GenerateAccessToken (
                    new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
                );
                var (refreshToken, refreshTokenExpires) = jwtService.GenerateRefreshToken();
                await authenticationRepository.SaveRefreshToken(transaction.DbTransaction, user.Id, refreshToken, refreshTokenExpires, cancellationToken);
                return (LoginUserResult.Success, new TokenResult(accessToken, refreshToken, accessTokenExpires));
            }
            return (LoginUserResult.WrongPassword, null);
        }
        catch (Exception e)
        {
            await transaction.Rollback();
            logger.LogError(e, "Unable to login user");
            return (LoginUserResult.UnknownError, null);
        }
    }

    public async Task<(RegisterUserResult, TokenResult?)> RegisterUser(string userName, string password, CancellationToken cancellationToken)
    {
        using var transaction = await transactionFactory.BeginTransaction();
        try
        {
            var user = await userRepository.CreateUser(transaction.DbTransaction, userName);
            if (user is null)
            {
                return (RegisterUserResult.UnableToCreateUser, null);
            }
            var hash = await passwordHasher.HashPassword(password);
            var authentication = new UserAuthentication
            {
                UserId = user.Id,
                Type = AuthenticationType.Password,
                AuthenticationType = new PasswordAuthentication(hash.Password, hash.Salt)
            };
            await authenticationRepository.AddAuthentication(transaction.DbTransaction, authentication);
            await fileNodeRepository.CreateRootNode(transaction.DbTransaction, user.Id);
            await operationFactory.CreateInitUserFileSystemOperation().Execute(transaction, user.Id, cancellationToken);
            var (accessToken, accessTokenExpires) = jwtService.GenerateAccessToken (
                new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
            );
            var (refreshToken, refreshTokenExpires) = jwtService.GenerateRefreshToken();
            await authenticationRepository.SaveRefreshToken(transaction.DbTransaction, user.Id, refreshToken, refreshTokenExpires, cancellationToken);
            await transaction.Commit(cancellationToken);
            return (RegisterUserResult.Success, new TokenResult(accessToken, refreshToken, accessTokenExpires));
        }
        catch (Exception e)
        {
            await transaction.Rollback();
            logger.LogError(e, "Unable to register user.");
            return (RegisterUserResult.UnableToCreateUser, null);
        }
    }
    
    public async Task<(RenewTokenResult, TokenResult?)> RenewAccessToken(string refreshToken, CancellationToken cancellationToken)
    {
        using var transaction = await transactionFactory.BeginTransaction();
        try
        {
            var user = await authenticationRepository.GetUserByRefreshToken(refreshToken, cancellationToken);
            if (user is null)
            {
                return (RenewTokenResult.TokenInvalid, null);
            }

            // Delete the old refresh token it can be used only once.
            await authenticationRepository.DeleteRefreshToken(refreshToken, cancellationToken);

            var (accessToken, accessTokenExpires) = jwtService.GenerateAccessToken (
                new Claim(ClaimTypes.NameIdentifier, user.Id.ToString(CultureInfo.InvariantCulture), ClaimValueTypes.Integer64)
            );
            var (newRefreshToken, refreshTokenExpires) = jwtService.GenerateRefreshToken();
            await authenticationRepository.SaveRefreshToken(transaction.DbTransaction, user.Id, newRefreshToken, refreshTokenExpires, cancellationToken);
            return (RenewTokenResult.Success, new TokenResult(accessToken, refreshToken, accessTokenExpires));
        }
        catch (Exception e)
        {
            await transaction.Rollback();
            logger.LogError(e, "Unable to renew access token.");
            return (RenewTokenResult.UnknownError, null);
        }
    }
}
