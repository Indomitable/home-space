using System.Text;
using HomeSpace.Infrastructure.Configuration;
using HomeSpace.Security.Configuration;
using HomeSpace.Security.Jwt;
using HomeSpace.Security.Password;
using HomeSpace.Security.Services;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.IdentityModel.Tokens;

namespace HomeSpace.Security;

public static class ServiceBuilder
{
    public static IServiceCollection AddHomeSpaceSecurity(this IServiceCollection serviceCollection, IConfiguration configuration)
    {
        serviceCollection.AddJwtAuthentication(configuration);
        serviceCollection.AddPasswordHashing();
        serviceCollection.AddSingleton<IAuthenticationService, AuthenticationService>();
        return serviceCollection;
    }
    
    private static void AddJwtAuthentication(this IServiceCollection serviceCollection, IConfiguration configuration)
    {
        serviceCollection.AddConfiguration<JwtConfiguration>("Security:Token");
        serviceCollection.AddSingleton<IJwtService, JwtService>();
        var jwtConfig = configuration.GetSection("Security:Token").Get<JwtConfiguration>();
        serviceCollection.AddAuthentication(o =>
            {
                o.DefaultAuthenticateScheme = JwtBearerDefaults.AuthenticationScheme;
                o.DefaultChallengeScheme = JwtBearerDefaults.AuthenticationScheme;
            })
            .AddJwtBearer(o =>
            {
                var key = new SymmetricSecurityKey(Encoding.ASCII.GetBytes(jwtConfig.TokenSecret));
                o.TokenValidationParameters = new TokenValidationParameters
                {
                    ValidateAudience = true,
                    ValidAudience = jwtConfig.Audience,
                    ValidateIssuer = true,
                    ValidIssuer = jwtConfig.Issuer,
                    ValidateIssuerSigningKey = true,
                    IssuerSigningKey = key,
                };
            });
    }

    private static void AddPasswordHashing(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddConfiguration<PasswordConfiguration>("Security:Password");
        serviceCollection.AddSingleton<IPasswordHasher, PasswordHasher>();
    }
}