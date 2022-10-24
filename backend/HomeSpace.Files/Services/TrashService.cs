namespace HomeSpace.Files.Services;

public interface ITrashService
{
    Task<string> MoveFileToTrash(long userId, string path, CancellationToken cancellationToken);
    Task<string> CopyFileVersionToTrash(long userId, string versionFile, CancellationToken cancellationToken);
}

internal sealed class TrashService : ITrashService
{
    private readonly IPathsService pathsService;
    private readonly IFileSystem fileSystem;

    public TrashService(IPathsService pathsService, IFileSystem fileSystem)
    {
        this.pathsService = pathsService;
        this.fileSystem = fileSystem;
    }
    
    public async Task<string> MoveFileToTrash(long userId, string path, CancellationToken cancellationToken)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, path);
        var trashFile = pathsService.GetTrashFile(userId);
        await fileSystem.MoveFile(absolutePath, trashFile, cancellationToken);
        return Path.GetFileName(trashFile);
    }

    public async Task<string> CopyFileVersionToTrash(long userId, string versionFile, CancellationToken cancellationToken)
    {
        var trashFilePath = pathsService.GetTrashFile(userId, versionFile);
        var versionFilePath = pathsService.GetVersionsFile(userId, versionFile);
        await fileSystem.CopyFile(versionFilePath, trashFilePath, cancellationToken);
        return versionFile;
    }
}