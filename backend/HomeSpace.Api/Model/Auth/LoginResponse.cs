using System.Text.Json.Serialization;

namespace HomeSpace.Api.Model.Auth;

public record LoginResponse(
    [property: JsonPropertyName("access_token")] string AccessToken,
    [property: JsonPropertyName("refresh_token")] string RefreshToken
);
