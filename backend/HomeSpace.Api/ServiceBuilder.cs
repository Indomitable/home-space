using HomeSpace.Api.Configuration;

namespace HomeSpace.Api;

public static class ServiceBuilder
{
    public static void EnableSwagger(this WebApplication app, IConfiguration configuration)
    {
        var swaggerConfig = configuration.GetSection("Swagger").Get<SwaggerConfiguration>();
        if (swaggerConfig.Enable)
        {
            app.UseSwagger();
            app.UseSwaggerUI();
        }
    }
}