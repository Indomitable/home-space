using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using HomeSpace.Security.Configuration;
using Microsoft.IdentityModel.Tokens;

namespace HomeSpace.Security.Jwt;

internal interface IJwtService
{
    string GenerateToken(params Claim[] claims);
    IReadOnlyList<Claim> GetTokenClaims(string token);
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

    public string GenerateToken(params Claim[] claims)
    {
        var tokenClaims = new List<Claim>(claims);
        var securityTokenDescriptor = new SecurityTokenDescriptor
        {
            Subject = new ClaimsIdentity(tokenClaims),
            Expires = DateTime.UtcNow.Add(configuration.ExpireTime),
            SigningCredentials = new SigningCredentials(key, SecurityAlgorithms.HmacSha512Signature),
            Issuer = configuration.Issuer,
            Audience = configuration.Audience
        };

        var jwtSecurityTokenHandler = new JwtSecurityTokenHandler();
        var securityToken = jwtSecurityTokenHandler.CreateToken(securityTokenDescriptor);
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