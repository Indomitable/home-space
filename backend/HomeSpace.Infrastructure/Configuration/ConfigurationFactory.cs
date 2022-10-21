using Microsoft.Extensions.Configuration;

namespace HomeSpace.Infrastructure.Configuration;

public static class ConfigurationFactory
{
    public static IConfiguration Create()
    {
        IConfigurationBuilder configurationBuilder = new ConfigurationManager();
        configurationBuilder.AddJsonFile("appsettings.json", false, false);
        var environment = Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT") ?? "Production";
        configurationBuilder.AddJsonFile($"appsettings.{environment}.json", true, false);
        configurationBuilder.AddEnvironmentVariables("ASPNETCORE_");
        return configurationBuilder.Build();
    }
}