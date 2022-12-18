using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

public record DeleteFolderUserOperation(long UserId, string Path) : IFileUserOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileUserOperation> logger, CancellationToken cancellationToken)
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

    public IRevertFileUserOperation CreateRevertOperation()
    {
        return new RevertDeleteFolderUserOperation(UserId, Path);
    }
}

public record RevertDeleteFolderUserOperation(long UserId, string Path) : IRevertFileUserOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileUserOperation> logger)
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