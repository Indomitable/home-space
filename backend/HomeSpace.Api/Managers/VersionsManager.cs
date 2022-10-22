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

        var user = currentUserProvider.RequireAuthorizedUser();
        var versionPath = await service.VersionFile(user.Id, node.FileSystemPath, cancellationToken);
        await repository.AddFileVersion(user.Id, node.Id, node.Version, node.Size, versionPath);
        return VersionNodeResult.Success;
    }
}