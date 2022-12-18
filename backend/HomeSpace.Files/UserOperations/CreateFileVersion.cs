using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

/// <summary>
/// Copy File on path to history folder with VersionFileName
/// </summary>
public record CreateFileUserVersion(long UserId, string Path, string VersionFileName): IFileUserOperation
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
            await fileSystem.CopyFile(fileAbsolutePath, versionAbsolutePath, cancellationToken);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to create file version. [UserId: {userId}][Path: {path}][Version: {versionFileName}]", UserId, Path, VersionFileName);
            return false;
        }
        return true;
    }

    public IRevertFileUserOperation CreateRevertOperation()
    {
        return new RevertFileUserVersion(UserId, VersionFileName, Path);
    }
}

/// <summary>
/// Move file with VersionFileName from history folder to Path 
/// </summary>
public record RevertFileUserVersion(long UserId, string VersionFileName, string Path): IRevertFileUserOperation
{
    public async ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileUserOperation> logger)
    {
        try
        {
            var fileAbsolutePath = pathsService.ResolveAbsolutePath(UserId, Path);
            var versionAbsolutePath = pathsService.GetVersionsFile(UserId, VersionFileName);
            // The important part is copy file
            await fileSystem.CopyFile(versionAbsolutePath, fileAbsolutePath, CancellationToken.None);

            try
            {
                fileSystem.DeleteFile(fileAbsolutePath);
            }
            catch (Exception e)
            {
                // Unable to delete the version file is not essential, the clean service will delete orphans. 
                logger.LogWarning(e, "Unable to delete file version after restore [Version: {versionFileName}]", VersionFileName);
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to revert file version. [UserId: {userId}][Version: {versionFileName}][Destination: {destination}]", UserId, VersionFileName, Path);
            return false;
        }
        return true;
    }
}
