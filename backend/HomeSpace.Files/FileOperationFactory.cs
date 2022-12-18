using HomeSpace.Files.FileOperations;
using HomeSpace.Files.Services;

namespace HomeSpace.Files;

public interface IFileOperationFactory
{
    IFileOperation CreateFolder(string absolutePath);
    IFileOperation CreateUserFolder(long userId, string path);
    IFileOperation CopyUserFile(long userId, string source, string destination);
}

public class FileOperationFactory : IFileOperationFactory
{
    private readonly IPathsService pathsService;

    public FileOperationFactory(IPathsService pathsService)
    {
        this.pathsService = pathsService;
    }

    public IFileOperation CreateFolder(string absolutePath)
    {
        return new CreateFolder(absolutePath);
    }

    public IFileOperation CreateUserFolder(long userId, string path)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, path);
        return CreateFolder(absolutePath);
    }

    public IFileOperation CopyUserFile(long userId, string source, string destination)
    {
        var sourceAbsolutePath = pathsService.ResolveAbsolutePath(userId, source);
        var destinationAbsolutePath = pathsService.ResolveAbsolutePath(userId, destination);
        return new CopyFileOperation(sourceAbsolutePath, destinationAbsolutePath);
    }
}
