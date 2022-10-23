namespace HomeSpace.Files.Services;

public interface IFileSystem
{
    Task CopyFile(string source, string destination, CancellationToken cancellationToken);
    Task MoveFile(string source, string destination, CancellationToken cancellationToken);
    Task DeleteFile(string target, CancellationToken cancellationToken);
    Task CopyDirectory(string source, string destination, CancellationToken cancellationToken);
    Task MoveDirectory(string source, string destination, CancellationToken cancellationToken);
    Task DeleteDir(string target, CancellationToken cancellationToken);
    void CreateDir(string target);
    Stream OpenReadFile(string target);
    Stream CreateFile(string target);
    Task<long> WriteFile(string absolutePath, Stream contents, CancellationToken cancellationToken);
    Task Rename(string source, string destination, CancellationToken cancellationToken);
}

internal sealed class FileSystem : IFileSystem
{
    public async Task CopyFile(string source, string destination, CancellationToken cancellationToken)
    {
        await using var sourceStream = OpenReadFile(source);
        await using var destinationStream = CreateFile(destination);
        await sourceStream.CopyToAsync(destinationStream, cancellationToken);
        await destinationStream.FlushAsync(cancellationToken);
    }
    
    public async Task MoveFile(string source, string destination, CancellationToken cancellationToken)
    {
        await CopyFile(source, destination, cancellationToken);
        await DeleteFile(source, cancellationToken);
    }

    public async Task DeleteFile(string target, CancellationToken cancellationToken)
    {
        await Task.Run(() => File.Delete(target), cancellationToken);
    }
    
    public async Task CopyDirectory(string source, string destination, CancellationToken cancellationToken)
    {
        // Create destination.
        CreateDir(destination);
        foreach (var dir in Directory.EnumerateDirectories(source))
        {
            // Fore each other directory. Copy it.
            var entrySource = Path.Join(source, dir);
            var entryDestination = Path.Join(destination, dir);
            await CopyDirectory(entrySource, entryDestination, cancellationToken);
        }

        foreach (var file in Directory.EnumerateFiles(source))
        {
            var entrySource = Path.Join(source, file);
            var entryDestination = Path.Join(destination, file);
            await CopyFile(entrySource, entryDestination, cancellationToken);
        }
    }

    public async Task MoveDirectory(string source, string destination, CancellationToken cancellationToken)
    {
        await CopyDirectory(source, destination, cancellationToken);
        await DeleteDir(source, cancellationToken);
    }
    
    public async Task DeleteDir(string target, CancellationToken cancellationToken)
    {
        await Task.Run(() => Directory.Delete(target, true), cancellationToken);
    }

    public void CreateDir(string target)
    {
        Directory.CreateDirectory(target);
    }

    public Stream OpenReadFile(string target)
    {
        return File.OpenRead(target);
    }
    
    public Stream CreateFile(string target)
    {
        return File.Create(target);
    }

    public Task Rename(string source, string destination, CancellationToken cancellationToken)
    {
        return Task.Run(() => File.Move(source, destination, true), cancellationToken);
    }
    
    public async Task<long> WriteFile(string absolutePath, Stream contents, CancellationToken cancellationToken)
    {
        await using var writeStream = File.Create(absolutePath);
        await contents.CopyToAsync(writeStream, cancellationToken);
        await writeStream.FlushAsync(cancellationToken);
        return writeStream.Length;
    }

}
