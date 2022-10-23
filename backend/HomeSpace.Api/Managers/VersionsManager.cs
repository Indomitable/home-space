using HomeSpace.Api.Model.Versions;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Services;

namespace HomeSpace.Api.Managers;

public interface IVersionsManager
{
    Task<VersionNodeResult> VersionNode(FileNode node, CancellationToken cancellationToken);
    Task CopyHistory(FileNode source, FileNode destination, CancellationToken cancellationToken);

    Task DeleteHistory(FileNode node, CancellationToken cancellationToken);
}

public class VersionsManager : IVersionsManager
{
    private readonly ICurrentUserProvider currentUserProvider;
    private readonly IVersionsService service;
    private readonly IVersionsRepository repository;

    public VersionsManager(ICurrentUserProvider currentUserProvider,
        IVersionsService service,
        IVersionsRepository repository)
    {
        this.currentUserProvider = currentUserProvider;
        this.service = service;
        this.repository = repository;
    }
    
    public async Task<VersionNodeResult> VersionNode(FileNode node, CancellationToken cancellationToken)
    {
        if (node.NodeType != NodeType.File)
        {
            return VersionNodeResult.NodeIsFolder;
        }

        var versionPath = await service.VersionFile(node.UserId, node.FileSystemPath, cancellationToken);
        await repository.AddFileVersion(node.UserId, node.Id, node.Version, node.Size, versionPath);        
        return VersionNodeResult.Success;
    }

    /// <summary>
    /// Copy File version from one node to another.
    /// We are not coping the files just the records in the database. 
    /// </summary>
    /// <param name="source"></param>
    /// <param name="destination"></param>
    /// <param name="cancellationToken"></param>
    public Task CopyHistory(FileNode source, FileNode destination, CancellationToken cancellationToken)
    {
        return repository.CopyFileHistory(source.UserId, source.Id, destination.UserId, destination.Id, cancellationToken);
    }

    public Task DeleteHistory(FileNode node, CancellationToken cancellationToken)
    {
        return repository.DeleteFileHistory(node.UserId, node.Id, cancellationToken);
    }
}