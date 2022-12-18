using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

public record MoveFileUserOperation(long UserId, string SourcePath, string DestinationPath): IFileUserOperation
{
    private bool fileCopied = false;
    private bool fileDeleted = false;
    
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileUserOperation> logger, CancellationToken cancellationToken)
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
            fileCopied = true;
            fileSystem.DeleteFile(sourceAbsolutePath);
            fileDeleted = true;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to move file. [UserId: {userId}][Source: {source}][Destination: {destination}]", UserId, SourcePath, DestinationPath);
            // Unable to copy file or cancel requested
            return false;
        }
        return true;
    }

    /// <summary>
    /// Revert operation of copy operation is to delete the destination file. 
    /// </summary>
    public IRevertFileUserOperation CreateRevertOperation()
    {
        return new RevertMoveFileUserOperation(UserId, SourcePath, DestinationPath, fileCopied, fileDeleted);
    }
}

public record RevertMoveFileUserOperation(long UserId, string SourcePath, string DestinationPath, bool FileCopied, bool FileDeleted) : IRevertFileUserOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileUserOperation> logger)
    {
        try
        {
            var sourceAbsolutePath = pathsService.ResolveAbsolutePath(UserId, SourcePath);
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, DestinationPath);

            if (FileDeleted)
            {
                // if we able to delete file -> Operation is completed. Copy destination to source.
                await fileSystem.CopyFile(destinationAbsolutePath, sourceAbsolutePath, CancellationToken.None);
            }

            try
            {
                fileSystem.DeleteFile(destinationAbsolutePath);
            }
            catch (Exception e)
            {
                logger.LogError(e, "Unable to revert move operation. Unable to delete destination [UserId: {userId}][Path: {destinationPath}]", UserId, DestinationPath);    
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert move operation. [UserId: {userId}][Source: {source}][Destination: {destination}]", UserId, SourcePath, DestinationPath);
            return false;
        }
        return true;
    }
}