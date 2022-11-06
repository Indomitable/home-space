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
    public static IServiceCollection AddHomeSpaceSecurity(this IServiceCollection serviceCollection,
        IConfiguration configuration)
    {
        serviceCollection.AddConfiguration<AuthConfiguration>("Auth");
        serviceCollection.AddJwtAuthentication(configuration);
        serviceCollection.AddPasswordHashing();
        serviceCollection.AddSingleton<IAuthenticationService, AuthenticationService>();
        serviceCollection.AddScoped<ICurrentUserProvider, CurrentUserProvider>();
        return serviceCollection;
    }

    private static void AddJwtAuthentication(this IServiceCollection serviceCollection, IConfiguration configuration)
    {
        var jwtConfig = configuration.GetSection("Security:Token").Get<JwtConfiguration>();
        serviceCollection.AddSingleton(jwtConfig);
        serviceCollection.AddSingleton<IJwtService, JwtService>();
        
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
                    ValidateLifetime = true,
                    IgnoreTrailingSlashWhenValidatingAudience = true,
                    ClockSkew = TimeSpan.FromSeconds(5),
                };
                // Get difference between refresh and access token expire time. 
                var refreshTokenExpireDiff = jwtConfig.RefreshTokenExpireTime - jwtConfig.AccessTokenExpireTime;
                o.Events = new JwtBearerEvents
                {
                    OnTokenValidated = context =>
                    {
                        var validTo = context.SecurityToken.ValidTo.ToUniversalTime();
                        if (validTo - DateTime.UtcNow <= TimeSpan.FromMinutes(1))
                        {
                            context.HttpContext.Response.Headers.Add("X-REFRESH-TOKEN", "1");
                        }
                        return Task.CompletedTask;
                    },
                    OnAuthenticationFailed = context =>
                    {
                        if (context.Exception is SecurityTokenExpiredException expiredException)
                        {
                            if (expiredException.Expires < DateTime.UtcNow + refreshTokenExpireDiff)
                            {
                                // access token has expired and is still before refresh token expiration.
                                context.HttpContext.Response.Headers.Add("X-EXPIRED-TOKEN", "1");
                            }
                        }
                        return Task.CompletedTask;
                    }
                };
            });
    }

    private static void AddPasswordHashing(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddConfiguration<PasswordConfiguration>("Security:Password");
        serviceCollection.AddSingleton<IPasswordHasher, PasswordHasher>();
    }
}