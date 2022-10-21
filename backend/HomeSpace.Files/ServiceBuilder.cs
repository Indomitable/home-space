using HomeSpace.Files.Configuration;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Configuration;
using Microsoft.Extensions.DependencyInjection;

namespace HomeSpace.Files;

public static class ServiceBuilder
{
    public static IServiceCollection AddHomeSpaceFiles(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddConfiguration<FilesConfiguration>("Storage:Files");
        serviceCollection.AddSingleton<IPathsManager, PathsManager>();
        return serviceCollection;
    }
}