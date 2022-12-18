using Autofac;
using HomeSpace.Files.Configuration;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Configuration;

namespace HomeSpace.Files;

public class FilesModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.AddConfiguration<FilesConfiguration>("Storage:Files");
        builder.RegisterType<PathsService>().As<IPathsService>().SingleInstance();
        builder.RegisterType<FileSystem>().As<IFileSystem>().SingleInstance();
        builder.RegisterType<FilesService>().As<IFilesService>().SingleInstance();
        builder.RegisterType<VersionsService>().As<IVersionsService>().SingleInstance();
        builder.RegisterType<TrashService>().As<ITrashService>().SingleInstance();
        builder.RegisterType<FileOperationFactory>().As<IFileOperationFactory>().SingleInstance();
    }
}
