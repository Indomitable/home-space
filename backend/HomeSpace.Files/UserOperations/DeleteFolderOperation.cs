using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

public record DeleteFolderOperation(long UserId, string Path) : IFileOperation
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
            fileSystem.DeleteEmptyDir(destinationAbsolutePath);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to delete folder. [UserId: {userId}][Path: {path}]", UserId, Path);
            // Unable to copy file or cancel requested
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }

    public IRevertFileOperation CreateRevertOperation()
    {
        return new RevertDeleteFolderOperation(UserId, Path);
    }
}

public record RevertDeleteFolderOperation(long UserId, string Path) : IRevertFileOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            fileSystem.CreateDir(destinationAbsolutePath);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert delete folder operation. Unable to create the folder. [UserId: {userId}][Path: {path}]", UserId, Path);
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }
}