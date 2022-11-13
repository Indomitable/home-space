using System.Text.Json.Serialization;
using HomeSpace.Security.Services;

namespace HomeSpace.Api.Model.Auth;

public record TokenResponse(
    [property: JsonPropertyName("access_token")]
    string AccessToken,
    [property: JsonPropertyName("refresh_token")]
    string RefreshToken,
    [property: JsonPropertyName("expires_in")]
    int ExpiresIn
)
{
    [JsonPropertyName("token_type")]
    public string TokenType = "Bearer";


    public static TokenResponse FromTokenResult(TokenResult tokenResult)
    {
        return new TokenResponse(tokenResult.AccessToken, tokenResult.RefreshToken, tokenResult.ExpiresIn);
    }
}
