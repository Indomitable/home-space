using System.Globalization;
using HomeSpace.Files.Configuration;

namespace HomeSpace.Files.Services;

public interface IPathsService
{
    string UserDirectory(long userId);
    string UserTrashDirectory(long userId);
    string UserVersionsDirectory(long userId);
    string UserDownloadsDirectory(long userId);
    string ResolveAbsolutePath(long userId, string relativePath);
    string ResolveRelativePath(long userId, string absolutePath);

    (string Absolute, string Relative) ResolvePaths(long userId, string parentPath, string name);
    string GetDownloadsFile(long userId);
    string GetVersionsFile(long userId);
    string GetVersionsFile(long userId, string fileName);
    string GetTrashFile(long userId);
    string GetTrashFile(long userId, string fileName);
    string UserUploadsDirectory(long userId);
    string UserSystemDirectory(long userId);
}

internal sealed class PathsService : IPathsService
{
    private const string SystemDir = ".system";
    private const string TrashDir = "trash";
    private const string VersionsDir = "versions";
    private const string DownloadsDir = "downloads";
    private const string UploadsDir = "uploads";
    
    private readonly string basePath;

    public PathsService(FilesConfiguration configuration)
    {
        basePath = configuration.BaseLocation;
    }

    public string UserDirectory(long userId) =>
        Path.Join(basePath, userId.ToString(CultureInfo.InvariantCulture));
    
    public string UserSystemDirectory(long userId) =>
        Path.Join(basePath, SystemDir, userId.ToString(CultureInfo.InvariantCulture));

    public string UserTrashDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), TrashDir);

    public string UserVersionsDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), VersionsDir);
    
    public string UserDownloadsDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), DownloadsDir);
    
    public string UserUploadsDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), UploadsDir);

    public string ResolveAbsolutePath(long userId, string relativePath) =>
        Path.Join(UserDirectory(userId), relativePath);

    public string ResolveRelativePath(long userId, string absolutePath)
    {
        return Path.Join("/", Path.GetRelativePath(UserDirectory(userId), absolutePath));
    }
    public (string Absolute, string Relative) ResolvePaths(long userId, string parentPath, string name)
    {
        var rootFolder = UserDirectory(userId);
        return (Path.Join(rootFolder, parentPath, name), Path.Join(parentPath, name));
    }

    public string GetDownloadsFile(long userId)
    {
        var fileName = Guid.NewGuid().ToString("N");
        return Path.Join(UserDownloadsDirectory(userId), fileName);
    }

    public string GetVersionsFile(long userId)
    {
        var fileName = Guid.NewGuid().ToString("N");
        return GetVersionsFile(userId, fileName);
    }

    public string GetVersionsFile(long userId, string fileName)
    {
        return Path.Join(UserVersionsDirectory(userId), fileName);
    }

    public string GetTrashFile(long userId)
    {
        var fileName = Guid.NewGuid().ToString("N");
        return GetTrashFile(userId, fileName);
    }

    public string GetTrashFile(long userId, string fileName)
    {
        return Path.Join(UserTrashDirectory(userId), fileName);
    }
}
