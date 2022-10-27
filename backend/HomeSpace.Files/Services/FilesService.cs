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

    Task UploadFileChunk(long userId, string fileId, Stream fileStream, int chunk, CancellationToken cancellationToken);
    Task<(Stream?, string? error)> GetUploadFileChunks(long userId, string id, int totalChunks, CancellationToken cancellationToken);
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

    public async Task UploadFileChunk(long userId, string fileId, Stream fileStream, int chunk, CancellationToken cancellationToken)
    {
        var uploadsPath = pathsService.UserUploadsDirectory(userId);
        var uploadsFolder = Path.Join(uploadsPath, fileId);
        fileSystem.CreateDir(uploadsFolder);
        var target = Path.Join(uploadsFolder, $"{chunk}.chunk");
        await fileSystem.WriteFile(target, fileStream, cancellationToken);
    }

    public async Task<(Stream?, string? error)> GetUploadFileChunks(long userId, string id, int totalChunks, CancellationToken cancellationToken)
    {
        var uploadsFolder = Path.Join(pathsService.UserUploadsDirectory(userId), id);
        var chunks = Directory.EnumerateFiles(uploadsFolder, "*.chunk")
            .OrderBy(x => Convert.ToInt32(Path.GetFileNameWithoutExtension(x)))
            .ToList();
        if (chunks.Count != totalChunks) {
            return (null, "Uploaded chunks not equal to the total file chunks");
        }
        if (chunks.Count == 1) {
            // When there is only one chunk return it no need to create concat file
            return (File.OpenRead(chunks[0]), null);
        }

        var fullFile = File.Open(Path.Join(uploadsFolder, "full"), FileMode.Create, FileAccess.ReadWrite); 
        foreach (var chunk in chunks)
        {
            await using var chunkStream = File.OpenRead(chunk);
            await chunkStream.CopyToAsync(fullFile, cancellationToken);
            await chunkStream.FlushAsync(cancellationToken);
        }

        fullFile.Position = 0;
        return (fullFile, null);
    }
}
