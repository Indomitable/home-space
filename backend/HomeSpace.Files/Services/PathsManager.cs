using System.Globalization;
using HomeSpace.Files.Configuration;

namespace HomeSpace.Files.Services;

public interface IPathsManager
{
    void InitUserFileSystem(long userId);
    string UserDirectory(long userId);
    string UserTrashDirectory(long userId);
    string UserVersionsDirectory(long userId);
    string UserTempDirectory(long userId);
    string ResolveAbsolutePath(long userId, string relativePath);
    (string Absolute, string Relative) ResolvePaths(long userId, string parentPath, string name);
}

internal sealed class PathsManager : IPathsManager
{
    private const string SystemDir = ".system";
    private const string TrashDir = "trash";
    private const string VersionsDir = "versions";
    private const string TempDir = "temp";
    
    private readonly FilesConfiguration configuration;

    public PathsManager(FilesConfiguration configuration)
    {
        this.configuration = configuration;
    }

    public void InitUserFileSystem(long userId)
    {
        Directory.CreateDirectory(UserDirectory(userId));
        Directory.CreateDirectory(UserSystemDirectory(userId));
        Directory.CreateDirectory(UserTrashDirectory(userId));
        Directory.CreateDirectory(UserVersionsDirectory(userId));
        Directory.CreateDirectory(UserTempDirectory(userId));
    }

    public string UserDirectory(long userId) => 
        Path.Join(configuration.BaseLocation, userId.ToString(CultureInfo.InvariantCulture));

    public string UserTrashDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), TrashDir);

    public string UserVersionsDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), VersionsDir);
    
    public string UserTempDirectory(long userId) =>
        Path.Join(UserSystemDirectory(userId), TempDir);

    public string ResolveAbsolutePath(long userId, string relativePath) =>
        Path.Join(UserDirectory(userId), relativePath);

    public (string Absolute, string Relative) ResolvePaths(long userId, string parentPath, string name)
    {
        var rootFolder = UserDirectory(userId);
        return (Path.Join(rootFolder, parentPath, name), Path.Join(parentPath, name));
    }

    private string UserSystemDirectory(long userId) =>
        Path.Join(UserDirectory(userId), SystemDir);
}