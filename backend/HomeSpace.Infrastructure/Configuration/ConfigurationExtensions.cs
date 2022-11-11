using Autofac;
using Microsoft.Extensions.Configuration;
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
    
    public static void AddConfiguration<T>(this ContainerBuilder builder, string key)
        where T : class
    {
        builder.Register<T>(p =>
        {
            var configuration = p.Resolve<IConfiguration>();
            var options = configuration.GetSection(key).Get<T>();
            if (options is null)
            {
                var logger = p.Resolve<ILogger<T>>();
                logger.LogError("Configuration {key} can not be mapped. Check service config!", key);
                throw new Exception($"Wrong configuration: {key}");
            }
            return options;
        }).SingleInstance();
    }
}