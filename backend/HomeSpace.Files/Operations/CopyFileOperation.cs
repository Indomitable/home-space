using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.Operations;

public record CopyFileOperation(long UserId, string SourcePath, string DestinationPath): IFileOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return false;
        }
        try
        {
            var sourceAbsolutePath = pathsService.ResolveAbsolutePath(UserId, SourcePath);
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, DestinationPath);
            await fileSystem.CopyFile(sourceAbsolutePath, destinationAbsolutePath, cancellationToken);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to copy file. [UserId: {userId}][Source: {source}][Destination: {destination}]", UserId, SourcePath, DestinationPath);
            // Unable to copy file or cancel requested
            return false;
        }
        return true;
    }

    /// <summary>
    /// Revert operation of copy operation is to delete the destination file. 
    /// </summary>
    public IRevertFileOperation CreateRevertOperation()
    {
        return new RevertCopyFileOperation(UserId, DestinationPath);
    }
}

public record RevertCopyFileOperation(long UserId, string Path) : IRevertFileOperation
{
    public ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            // Copied or partial copied.
            fileSystem.DeleteFile(destinationAbsolutePath);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert copy operation. Unable to delete copied file. [UserId: {userId}][Path: {path}]", UserId, Path);
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }
}