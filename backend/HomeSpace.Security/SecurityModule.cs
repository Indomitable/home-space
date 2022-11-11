using Autofac;
using HomeSpace.Infrastructure.Configuration;
using HomeSpace.Security.Configuration;
using HomeSpace.Security.Password;
using HomeSpace.Security.Services;

namespace HomeSpace.Security;

public class SecurityModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.AddConfiguration<AuthConfiguration>("Auth");
        AddPasswordHashing(builder);
        builder.RegisterType<AuthenticationService>().As<IAuthenticationService>().SingleInstance();
        builder.RegisterType<CurrentUserProvider>().As<ICurrentUserProvider>().InstancePerLifetimeScope();
    }

    private static void AddPasswordHashing(ContainerBuilder builder)
    {
        builder.AddConfiguration<PasswordConfiguration>("Security:Password");
        builder.RegisterType<PasswordHasher>().As<IPasswordHasher>().SingleInstance();
    }
}