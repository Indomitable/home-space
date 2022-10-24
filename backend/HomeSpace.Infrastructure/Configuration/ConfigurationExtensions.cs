using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Infrastructure.Configuration;

public static class ConfigurationExtensions
{
    public static IConfigurationBuilder AddConfig(this IConfigurationBuilder configurationBuilder,
        IConfiguration configuration)
    {
        configurationBuilder.Sources.Clear();
        configurationBuilder.AddConfiguration(configuration);
        return configurationBuilder;
    }
    
    public static void AddConfiguration<T>(this IServiceCollection services, string key)
        where T : class
    {
        services.AddSingleton<T>(p =>
        {
            var configuration = p.GetRequiredService<IConfiguration>();
            var options = configuration.GetSection(key).Get<T>();
            if (options is null)
            {
                var logger = p.GetRequiredService<ILogger<T>>();
                logger.LogError("Configuration {key} can not be mapped. Check service config!", key);
                throw new Exception($"Wrong configuration: {key}");
            }
            return options;
        });
    }
}