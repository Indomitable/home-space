using Autofac;
using HomeSpace.Database.Configuration;
using HomeSpace.Database.Repository;
using HomeSpace.Infrastructure.Configuration;

namespace HomeSpace.Database;

public class DatabaseModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.AddConfiguration<DbConfiguration>("Storage:DB");
        builder.RegisterType<DbCommandFactory>().As<IDbCommandFactory>().SingleInstance();
        builder.RegisterType<DbAccess>().As<IDbAccess>().SingleInstance();
        builder.RegisterType<UserRepository>().As<IUserRepository>().SingleInstance();
        builder.RegisterType<AuthenticationRepository>().As<IAuthenticationRepository>().SingleInstance();
        builder.RegisterType<FileNodeRepository>().As<IFileNodeRepository>().SingleInstance();
        builder.RegisterType<VersionsRepository>().As<IVersionsRepository>().SingleInstance();
        builder.RegisterType<FavoritesRepository>().As<IFavoritesRepository>().SingleInstance();
        builder.RegisterType<TrashRepository>().As<ITrashRepository>().SingleInstance();
    }
}