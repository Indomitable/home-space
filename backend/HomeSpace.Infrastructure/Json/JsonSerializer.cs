using System.Text.Json;
using System.Text.Json.Serialization;

namespace HomeSpace.Infrastructure.Json;

public interface IJsonSerializer
{
    JsonSerializerOptions Options { get; }
    
    string Serialize<TValue>(TValue obj);
    TValue? Deserialize<TValue>(string value);
}

internal sealed class JsonSerializer : IJsonSerializer
{
    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Converters =
        {
            new JsonStringEnumConverter(),
        }
    };

    public JsonSerializerOptions Options => SerializerOptions;
    
    public string Serialize<TValue>(TValue obj)
    {
        return System.Text.Json.JsonSerializer.Serialize(obj, SerializerOptions);
    }
    
    public TValue? Deserialize<TValue>(string value)
    {
        return System.Text.Json.JsonSerializer.Deserialize<TValue>(value, SerializerOptions);
    }
}
