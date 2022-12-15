using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.Operations;

public record RestoreFileFromGarbageOperation(long UserId, string Path, string TrashFileName): IFileOperation
{
    private bool fileCopied = false;
    private bool fileDeleted = false;
    
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return false;
        }
        try
        {
            var sourceAbsolutePath = pathsService.GetTrashFile(UserId, TrashFileName);
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            await fileSystem.CopyFile(sourceAbsolutePath, destinationAbsolutePath, cancellationToken);
            fileCopied = true;
            fileSystem.DeleteFile(sourceAbsolutePath);
            fileDeleted = true;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to restore file to trash. [UserId: {userId}][TrashFileName: {source}][Destination: {destination}]", UserId, TrashFileName, Path);
            // Unable to copy file or cancel requested
            return false;
        }
        return true;
    }

    public IRevertFileOperation CreateRevertOperation()
    {
        return new RevertRestoreFileToGarbageOperation(UserId, Path, TrashFileName, fileCopied, fileDeleted);
    }
}

public record RevertRestoreFileToGarbageOperation(long UserId, string Path, string TrashFileName, bool FileCopied, bool FileDeleted) : IRevertFileOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            var sourceAbsolutePath = pathsService.GetTrashFile(UserId, TrashFileName);
            var destinationAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);

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
                logger.LogError(e, "Unable to revert restore from trash operation. Unable to delete destination [UserId: {userId}][TrashFileName: {trashFileName}]", UserId, TrashFileName);    
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert restore from trash operation. [UserId: {userId}][Source: {source}][TrashFileName: {trashFileName}]", UserId, Path, TrashFileName);
            return false;
        }
        return true;
    }
}