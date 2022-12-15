using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.Operations;

public record CreateFolderOperation(long UserId, string Path) : IFileOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return ValueTask.FromResult(false);
        }
        try
        {
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            fileSystem.CreateDir(destinationAbsolutePath);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to create folder. [UserId: {userId}][Path: {path}]", UserId, Path);
            // Unable to copy file or cancel requested
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }

    public IRevertFileOperation CreateRevertOperation()
    {
        return new RevertCreateFolderOperation(UserId, Path);
    }
}

public record RevertCreateFolderOperation(long UserId, string Path) : IRevertFileOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            fileSystem.DeleteEmptyDir(destinationAbsolutePath);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert create folder operation. Unable to delete the folder. [UserId: {userId}][Path: {path}]", UserId, Path);
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }
}