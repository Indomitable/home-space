using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using HomeSpace.Security.Configuration;
using Microsoft.IdentityModel.Tokens;

namespace HomeSpace.Security.Jwt;

public interface IJwtService
{
    IReadOnlyList<Claim> GetTokenClaims(string token);
    (string token, int expiresIn) GenerateAccessToken(params Claim[] claims);
    (string token, DateTime expires) GenerateRefreshToken();
}

internal sealed class JwtService : IJwtService
{
    private readonly JwtConfiguration configuration;
    private readonly SymmetricSecurityKey key;
    private readonly TokenValidationParameters validationParameters;

    public JwtService(JwtConfiguration configuration)
    {
        this.configuration = configuration;
        key = new SymmetricSecurityKey(Encoding.ASCII.GetBytes(configuration.TokenSecret));
        validationParameters = new TokenValidationParameters
        {
            ValidateAudience = true,
            ValidAudience = configuration.Audience,
            ValidateIssuer = true,
            ValidIssuer = configuration.Issuer,
            ValidateIssuerSigningKey = true,
            IssuerSigningKey = key
        };
    }

    public (string token, int expiresIn) GenerateAccessToken(params Claim[] claims)
    {
        var expires = DateTime.UtcNow.Add(configuration.AccessTokenExpireTime);
        return (GenerateToken(expires, claims), (int)configuration.AccessTokenExpireTime.TotalSeconds);
    }
    
    public (string token, DateTime expires) GenerateRefreshToken()
    {
        var expires = DateTime.UtcNow.Add(configuration.RefreshTokenExpireTime);
        return (GenerateToken(expires, Array.Empty<Claim>()), expires);
    }

    private string GenerateToken(DateTime expires, IEnumerable<Claim> claims)
    {
        var tokenClaims = new List<Claim>(claims);
        var securityTokenDescriptor = new SecurityTokenDescriptor
        {
            Subject = new ClaimsIdentity(tokenClaims),
            Expires = expires,
            SigningCredentials = new SigningCredentials(key, SecurityAlgorithms.HmacSha512Signature),
            Issuer = configuration.Issuer,
            Audience = configuration.Audience
        };

        var jwtSecurityTokenHandler = new JwtSecurityTokenHandler();
        var securityToken = jwtSecurityTokenHandler.CreateJwtSecurityToken(securityTokenDescriptor);
        return jwtSecurityTokenHandler.WriteToken(securityToken);
    }

    public IReadOnlyList<Claim> GetTokenClaims(string token)
    {
        if (string.IsNullOrEmpty(token))
        {
            throw new ArgumentException("Invalid token");
        }

        var jwtSecurityTokenHandler = new JwtSecurityTokenHandler();
        var tokenValid = jwtSecurityTokenHandler.ValidateToken(token, validationParameters, out _);
        return tokenValid.Claims.ToList();
    }
}