namespace HomeSpace.Security.Configuration;

public sealed class JwtConfiguration
{
    public string TokenSecret { get; set; } = string.Empty;
    public string Audience { get; set; } = string.Empty;
    public string Issuer { get; set; } = string.Empty;

    public TimeSpan AccessTokenExpireTime { get; set; } = TimeSpan.FromMinutes(5);
    public TimeSpan RefreshTokenExpireTime { get; set; } = TimeSpan.FromMinutes(7);
}