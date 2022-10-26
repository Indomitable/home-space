using System.IO.Compression;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Files.Services;

public interface IFilesService
{
    Stream ReadFile(long userId, string relativePath);
    
    Stream ZipFolder(long userId, string relativePath);
    (string Absolute, string Relative) CreateFolder(long userId, string parentPath, string name);

    Task<long> CreateFile(long userId, string path, Stream contents, CancellationToken cancellationToken);
    
    Task DeleteFile(long userId, string path, CancellationToken cancellationToken);
    
    Task DeleteFolder(long userId, string path, CancellationToken cancellationToken);
    
    /// <summary>
    /// Renames user file located on path and returns the new path
    /// </summary>
    /// <param name="userId">UserId</param>
    /// <param name="path">Relative path</param>
    /// <param name="name">New file name</param>
    /// <param name="cancellationToken"></param>
    /// <returns></returns>
    Task<string> Rename(long userId, string path, string name, NodeType nodeType, CancellationToken cancellationToken);
}

internal sealed class FilesService : IFilesService
{
    private readonly IPathsService pathsService;
    private readonly IFileSystem fileSystem;

    public FilesService(IPathsService pathsService, IFileSystem fileSystem)
    {
        this.pathsService = pathsService;
        this.fileSystem = fileSystem;
    }
    
    public Stream ReadFile(long userId, string relativePath)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, relativePath);
        return fileSystem.OpenReadFile(absolutePath);
    }

    public Stream ZipFolder(long userId, string relativePath)
    {
        var temporaryFile = pathsService.GetTemporaryFile(userId);
        var absolutePath = pathsService.ResolveAbsolutePath(userId, relativePath);
        ZipFile.CreateFromDirectory(absolutePath, temporaryFile, CompressionLevel.Optimal, true);
        return fileSystem.OpenReadFile(temporaryFile);
    }

    public (string Absolute, string Relative) CreateFolder(long userId, string parentPath, string name)
    {
        var paths = pathsService.ResolvePaths(userId, parentPath, name);
        fileSystem.CreateDir(paths.Absolute);
        return paths;
    }

    public async Task<long> CreateFile(long userId, string path, Stream contents, CancellationToken cancellationToken)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, path);
        return await fileSystem.WriteFile(absolutePath, contents, cancellationToken);
    }

    public async Task DeleteFile(long userId, string path, CancellationToken cancellationToken)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, path);
        await fileSystem.DeleteFile(absolutePath, cancellationToken);
    }

    public async Task DeleteFolder(long userId, string path, CancellationToken cancellationToken)
    {
        var absolutePath = pathsService.ResolveAbsolutePath(userId, path);
        await fileSystem.DeleteDir(absolutePath, cancellationToken);
    }

    public async Task<string> Rename(long userId, string path, string name, NodeType nodeType, CancellationToken cancellationToken)
    {
        var sourcePath = pathsService.ResolveAbsolutePath(userId, path);
        var destination = Path.Join(Path.GetDirectoryName(sourcePath), name);
        if (nodeType == NodeType.File) {
            await fileSystem.RenameFile(sourcePath, destination, cancellationToken);
        } else {
            await fileSystem.RenameFolder(sourcePath, destination, cancellationToken);
        }
        return pathsService.ResolveRelativePath(userId, destination);
    }
}
