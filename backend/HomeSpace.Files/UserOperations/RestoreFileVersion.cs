using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

/// <summary>
/// Copy version file over path.
/// </summary>
public record RestoreFileUserVersion(long UserId, string Path, string VersionFileName): IFileUserOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileUserOperation> logger, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return false;
        }
        try
        {
            var fileAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            var versionAbsolutePath = pathsService.GetVersionsFile(UserId, VersionFileName);
            await fileSystem.CopyFile(versionAbsolutePath, fileAbsolutePath, cancellationToken);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to restore file version. [UserId: {userId}][Path: {path}][Version: {versionFileName}]", UserId, Path, VersionFileName);
            return false;
        }
        return true;
    }

    /// <summary>
    /// Revert restore file version is not supported.
    /// Revert is done with another restore file version command which restore previous file if has any. 
    /// </summary>
    /// <returns></returns>
    /// <exception cref="NotSupportedException"></exception>
    public IRevertFileUserOperation CreateRevertOperation()
    {
        throw new NotSupportedException();
    }
}

