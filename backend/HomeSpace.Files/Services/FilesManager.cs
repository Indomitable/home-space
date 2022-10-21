namespace HomeSpace.Files.Services;

public interface IFilesManager
{
    Stream ReadFile(long userId, string relativePath);
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

    public (string Absolute, string Relative) CreateFolder(long userId, string parentPath, string name)
    {
        var paths = pathsManager.ResolvePaths(userId, parentPath, name);
        Directory.CreateDirectory(paths.Absolute);
        return paths;
    }
}
