namespace HomeSpace.Security.Configuration;

public sealed class JwtConfiguration
{
    public string TokenSecret { get; set; } = string.Empty;
    public string Audience { get; set; } = string.Empty;
    public string Issuer { get; set; } = string.Empty;

    public TimeSpan ExpireTime { get; set; } = TimeSpan.FromMinutes(5);
}