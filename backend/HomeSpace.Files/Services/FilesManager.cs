using System.IO.Compression;

namespace HomeSpace.Files.Services;

public interface IFilesManager
{
    Stream ReadFile(long userId, string relativePath);
    
    Stream ZipFolder(long userId, string relativePath);
    (string Absolute, string Relative) CreateFolder(long userId, string parentPath, string name);
}

public class FilesManager : IFilesManager
{
    private readonly IPathsManager pathsManager;

    public FilesManager(IPathsManager pathsManager)
    {
        this.pathsManager = pathsManager;
    }
    
    public Stream ReadFile(long userId, string relativePath)
    {
        var absolutePath = pathsManager.ResolveAbsolutePath(userId, relativePath);
        return File.OpenRead(absolutePath);
    }

    public Stream ZipFolder(long userId, string relativePath)
    {
        var temporaryFile = pathsManager.GetTemporaryFile(userId);
        var absolutePath = pathsManager.ResolveAbsolutePath(userId, relativePath);
        ZipFile.CreateFromDirectory(absolutePath, temporaryFile, CompressionLevel.Optimal, true);
        return File.OpenRead(temporaryFile);
    }

    public (string Absolute, string Relative) CreateFolder(long userId, string parentPath, string name)
    {
        var paths = pathsManager.ResolvePaths(userId, parentPath, name);
        Directory.CreateDirectory(paths.Absolute);
        return paths;
    }
}
