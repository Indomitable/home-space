using System.Text.Json.Serialization;

namespace HomeSpace.Api.Model.Auth;

public record RegisterResponse([property: JsonPropertyName("access_token")] string Token);