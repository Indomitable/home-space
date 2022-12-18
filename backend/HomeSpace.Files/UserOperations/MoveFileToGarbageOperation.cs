using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

public record MoveFileUserToGarbageOperation(long UserId, string Path, string TrashFileName): IFileUserOperation
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
            var sourceAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            var destinationAbsolutePath = pathsService.GetTrashFile(UserId, TrashFileName);
            await fileSystem.CopyFile(sourceAbsolutePath, destinationAbsolutePath, cancellationToken);
            fileCopied = true;
            fileSystem.DeleteFile(sourceAbsolutePath);
            fileDeleted = true;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to move file to trash. [UserId: {userId}][Source: {source}][TrashFileName: {destination}]", UserId, Path, TrashFileName);
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
        return new RevertMoveFileUserToGarbageOperation(UserId, Path, TrashFileName, fileCopied, fileDeleted);
    }
}

public record RevertMoveFileUserToGarbageOperation(long UserId, string Path, string TrashFileName, bool FileCopied, bool FileDeleted) : IRevertFileUserOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileUserOperation> logger)
    {
        try
        {
            var sourceAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            var destinationAbsolutePath = pathsService.GetTrashFile(UserId, TrashFileName);

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
                logger.LogError(e, "Unable to revert move to trash operation. Unable to delete destination [UserId: {userId}][TrashFileName: {trashFileName}]", UserId, TrashFileName);    
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert move operation. [UserId: {userId}][Source: {source}][TrashFileName: {trashFileName}]", UserId, Path, TrashFileName);
            return false;
        }
        return true;
    }
}