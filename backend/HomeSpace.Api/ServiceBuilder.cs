using Microsoft.AspNetCore.Diagnostics;

namespace HomeSpace.Api;

public static partial class ServiceBuilder
{
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