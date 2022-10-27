using FluentValidation;
using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Auth;
using HomeSpace.Api.Model.Files;
using HomeSpace.Api.Validations;
using Microsoft.AspNetCore.Diagnostics;
using Microsoft.AspNetCore.StaticFiles;

namespace HomeSpace.Api;

public static partial class ServiceBuilder
{
    public static IServiceCollection AddServices(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddScoped<IContentTypeProvider>(sp => new FileExtensionContentTypeProvider());
        serviceCollection.AddScoped<IFilesManager, FilesManager>();
        serviceCollection.AddScoped<IVersionsManager, VersionsManager>();
        serviceCollection.AddScoped<IFavoritesManager, FavoritesManager>();
        serviceCollection.AddScoped<ITrashManager, TrashManager>();
        
        serviceCollection.AddValidations();
        return serviceCollection;
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

    private static void AddValidations(this IServiceCollection serviceCollection)
    {
        serviceCollection.AddScoped<IValidator<LoginRequest>, LoginRequestValidator>();
        serviceCollection.AddScoped<IValidator<RegisterRequest>, RegisterRequestValidator>();
        
        serviceCollection.AddScoped<IValidator<RenameNodeRequest>, RenameNodeRequestValidator>();
        serviceCollection.AddScoped<IValidator<CreateFolderRequest>, CreateFolderRequestValidator>();
        serviceCollection.AddScoped<IValidator<GetFilesRequest>, GetFilesRequestValidator>();

        serviceCollection.AddScoped<IValidator<UploadFileRequest>, UploadFileRequestValidator>();
        
        serviceCollection.AddScoped<IValidator<CopyNodeRequest>, CopyNodeRequestValidator>();
        serviceCollection.AddScoped<IValidator<MoveNodeRequest>, MoveNodeRequestValidator>();
    }
}