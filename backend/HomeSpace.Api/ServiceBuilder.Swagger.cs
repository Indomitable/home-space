using HomeSpace.Api.Configuration;
using Microsoft.OpenApi.Models;

namespace HomeSpace.Api;

public static partial class ServiceBuilder
{
    public static IServiceCollection AddSwagger(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddSwaggerGen(options =>
        {
            var securityScheme = new OpenApiSecurityScheme()
            {
                Description = "JWT",
                Name = "Authorization",
                In = ParameterLocation.Header,
                Type = SecuritySchemeType.OAuth2,
                Scheme = "OAuth2",
                Reference = new OpenApiReference
                {   
                    Id = "OAuth2",
                    Type = ReferenceType.SecurityScheme
                },
                Flows = new OpenApiOAuthFlows
                {
                    Password = new OpenApiOAuthFlow
                    {
                        Scopes = new Dictionary<string, string>(),
                        TokenUrl = new Uri("/api/auth/login-auth", UriKind.Relative),
                    }
                }
            };
            options.AddSecurityDefinition(securityScheme.Reference.Id, securityScheme);

            options.AddSecurityRequirement(new OpenApiSecurityRequirement
            {
                { securityScheme, new List<string>() }
            });
        });
        return serviceCollection;
    }
    
    public static void EnableSwagger(this WebApplication app, IConfiguration configuration)
    {
        var swaggerConfig = configuration.GetSection("Swagger").Get<SwaggerConfiguration>();
        if (swaggerConfig.Enable)
        {
            app.UseSwagger(options =>
            {
                options.PreSerializeFilters.Add((swagger, httpReq) =>
                {
                    swagger.Servers = new List<OpenApiServer> { new() { Url = $"{httpReq.Scheme}://{httpReq.Host.Value}" } };
                });
            });
            app.UseSwaggerUI();
        }
    }
}