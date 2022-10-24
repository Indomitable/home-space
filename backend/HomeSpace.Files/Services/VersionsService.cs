namespace HomeSpace.Files.Services;

public interface IVersionsService
{
    Task<string> VersionFile(long userId, string relativePath, CancellationToken cancellationToken);
}

internal sealed class VersionsService : IVersionsService
{
    private readonly IPathsService pathsService;
    private readonly IFileSystem fileSystem;

    public VersionsService(IPathsService pathsService, IFileSystem fileSystem)
    {
        this.pathsService = pathsService;
        this.fileSystem = fileSystem;
    }

    public async Task<string> VersionFile(long userId, string relativePath, CancellationToken cancellationToken)
    {
        var destination = pathsService.GetVersionsFile(userId);
        var sourceFile = pathsService.ResolveAbsolutePath(userId, relativePath);
        await fileSystem.CopyFile(sourceFile, destination, cancellationToken);
        return Path.GetFileName(destination);
    }
}