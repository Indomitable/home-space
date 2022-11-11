using Autofac;
using FluentValidation;
using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Auth;
using HomeSpace.Api.Model.Files;
using HomeSpace.Api.Validations;
using Microsoft.AspNetCore.StaticFiles;

namespace HomeSpace.Api;

public class ApiModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.RegisterType<FileExtensionContentTypeProvider>().As<IContentTypeProvider>().InstancePerLifetimeScope();
        builder.RegisterType<FilesManager>().As<IFilesManager>().InstancePerLifetimeScope();
        builder.RegisterType<VersionsManager>().As<IVersionsManager>().InstancePerLifetimeScope();
        builder.RegisterType<FavoritesManager>().As<IFavoritesManager>().InstancePerLifetimeScope();
        AddValidations(builder);
    }
    
    private static void AddValidations(ContainerBuilder builder)
    {
        builder.RegisterType<LoginRequestValidator>().As<IValidator<LoginRequest>>().InstancePerLifetimeScope();
        builder.RegisterType<RegisterRequestValidator>().As<IValidator<RegisterRequest>>().InstancePerLifetimeScope();
        
        builder.RegisterType<RenameNodeRequestValidator>().As<IValidator<RenameNodeRequest>>().InstancePerLifetimeScope();
        builder.RegisterType<CreateFolderRequestValidator>().As<IValidator<CreateFolderRequest>>().InstancePerLifetimeScope();
        builder.RegisterType<GetFilesRequestValidator>().As<IValidator<GetFilesRequest>>().InstancePerLifetimeScope();
    
        builder.RegisterType<UploadFileChunkRequestValidator>().As<IValidator<UploadFileChunkRequest>>().InstancePerLifetimeScope();
        builder.RegisterType<UploadLastFileChunkRequestValidator>().As<IValidator<UploadLastFileChunkRequest>>().InstancePerLifetimeScope();
        
        builder.RegisterType<CopyNodeRequestValidator>().As<IValidator<CopyNodeRequest>>().InstancePerLifetimeScope();
        builder.RegisterType<MoveNodeRequestValidator>().As<IValidator<MoveNodeRequest>>().InstancePerLifetimeScope();
    }
}