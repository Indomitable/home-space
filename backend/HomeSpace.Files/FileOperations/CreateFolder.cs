using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.FileOperations;

public record CreateFolder(string AbsolutePath) : IFileOperation
{
    public Task Execute(IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        try
        {
            fileSystem.CreateDir(AbsolutePath);
            return Task.CompletedTask;
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to create folder operation. [Path: {path}]", AbsolutePath);
            throw;
        }
    }

    public IRevertFileOperation CreateRevertOperation()
    {
        return new ReverseCreateFolder(AbsolutePath);
    }
}

public record ReverseCreateFolder(string AbsolutePath): IRevertFileOperation
{
    public ValueTask<bool> Execute(IFileSystem fileSystem, ILogger<IRevertFileOperation> logger)
    {
        try
        {
            fileSystem.DeleteEmptyDir(AbsolutePath);
            return ValueTask.FromResult(true);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert create folder operation. [Path: {path}]", AbsolutePath);
            return ValueTask.FromResult(false);
        }
    }
}
