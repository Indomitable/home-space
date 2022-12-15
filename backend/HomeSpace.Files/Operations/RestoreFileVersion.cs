using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.Operations;

/// <summary>
/// Copy version file over path.
/// </summary>
public record RestoreFileVersion(long UserId, string Path, string VersionFileName): IFileOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken)
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
    public IRevertFileOperation CreateRevertOperation()
    {
        throw new NotSupportedException();
    }
}

