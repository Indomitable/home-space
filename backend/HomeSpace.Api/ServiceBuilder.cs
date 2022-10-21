using HomeSpace.Api.Configuration;
using HomeSpace.Api.Managers;
using Microsoft.AspNetCore.Diagnostics;
using Microsoft.OpenApi.Models;

namespace HomeSpace.Api;

public static class ServiceBuilder
{
    public static IServiceCollection AddServices(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddScoped<IFilesManager, FilesManager>();
        return serviceCollection;
    }

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

    public static void HandleExceptions(this WebApplication app)
    {
        var logger = app.Services.GetRequiredService<ILogger<ExceptionHandlerFeature>>();
        app.UseExceptionHandler(new ExceptionHandlerOptions
        {
            ExceptionHandler = async context =>
            {
                var feature = context.Features.Get<IExceptionHandlerFeature>();
                if (feature is not null)
                {
                    logger.LogError(feature.Error, "Exception has occured. [Path: {path}]", feature.Path);
                }
                await context.Response.WriteAsJsonAsync(new { Message = "Exception occured. Please check the logs" });
            },
            AllowStatusCode404Response = true
        });
    }
}