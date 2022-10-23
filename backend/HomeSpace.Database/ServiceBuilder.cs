using HomeSpace.Database.Configuration;
using HomeSpace.Database.Repository;
using HomeSpace.Infrastructure.Configuration;
using Microsoft.Extensions.DependencyInjection;

namespace HomeSpace.Database;

public static class ServiceBuilder
{
    public static IServiceCollection AddHomeSpaceDb(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddConfiguration<DbConfiguration>("Storage:DB");
        serviceCollection.AddSingleton<IDbCommandFactory, DbCommandFactory>();
        serviceCollection.AddSingleton<IDbAccess, DbAccess>();
        serviceCollection.AddSingleton<IUserRepository, UserRepository>();
        serviceCollection.AddSingleton<IAuthenticationRepository, AuthenticationRepository>();
        serviceCollection.AddSingleton<IFileNodeRepository, FileNodeRepository>();
        serviceCollection.AddSingleton<IVersionsRepository, VersionsRepository>();
        serviceCollection.AddSingleton<IFavoritesRepository, FavoritesRepository>();
        return serviceCollection;
    }
}