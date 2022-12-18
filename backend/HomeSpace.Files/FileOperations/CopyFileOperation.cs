using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.FileOperations;

public record CopyFileOperation(string Source, string Destination): IFileOperation
{
    public async Task Execute(IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        try
        {
            await fileSystem.CopyFile(Source, Destination, cancellationToken);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to copy file. [Source: {source}][Destination: {destination}]", Source, Destination);
            throw;
        }
    }

    /// <summary>
    /// Revert operation of copy operation is to delete the destination file. 
    /// </summary>
    public IRevertFileOperation CreateRevertOperation()
    {
        return new RevertCopyFileOperation(Destination);
    }
}

public record RevertCopyFileOperation(string Path) : IRevertFileOperation
{
    public ValueTask<bool> Execute(IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            // Copied or partial copied.
            fileSystem.DeleteFile(Path);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert copy operation. Unable to delete copied file. [Path: {path}]", Path);
            return ValueTask.FromResult(false);
        }
        return ValueTask.FromResult(true);
    }
}
