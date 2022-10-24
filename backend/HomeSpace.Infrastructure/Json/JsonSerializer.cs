using System.Text.Json;
using System.Text.Json.Serialization;

namespace HomeSpace.Infrastructure.Json;

public interface IJsonSerializer
{
    string Serialize<TValue>(TValue obj);
    TValue? Deserialize<TValue>(string value);
}

public sealed class JsonSerializer : IJsonSerializer
{
    private static readonly JsonSerializerOptions Options = new JsonSerializerOptions();
        
    static JsonSerializer()
    {
        Configure(Options);
    }
    
    public static void Configure(JsonSerializerOptions options)
    {
        options.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
        options.DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull;
        options.Converters.Add(new JsonStringEnumConverter());
    }

    public string Serialize<TValue>(TValue obj)
    {
        return System.Text.Json.JsonSerializer.Serialize(obj, Options);
    }
    
    public TValue? Deserialize<TValue>(string value)
    {
        return System.Text.Json.JsonSerializer.Deserialize<TValue>(value, Options);
    }
}
