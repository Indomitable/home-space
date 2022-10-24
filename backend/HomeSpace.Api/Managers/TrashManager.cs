using HomeSpace.Database;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Services;

namespace HomeSpace.Api.Managers;

public interface ITrashManager
{
    Task MoveToTrash(long id, CancellationToken cancellationToken);
}

internal sealed class TrashManager : ITrashManager
{
    private readonly ICurrentUserProvider userProvider;
    private readonly IFileNodeRepository fileNodeRepository;
    private readonly ITrashRepository trashRepository;
    private readonly IVersionsRepository versionsRepository;
    private readonly ITrashService trashService;

    public TrashManager(ICurrentUserProvider userProvider, 
        IFileNodeRepository fileNodeRepository,
        ITrashRepository trashRepository,
        IVersionsRepository versionsRepository,
        ITrashService trashService)
    {
        this.userProvider = userProvider;
        this.fileNodeRepository = fileNodeRepository;
        this.trashRepository = trashRepository;
        this.versionsRepository = versionsRepository;
        this.trashService = trashService;
    }

    public async Task MoveToTrash(long id, CancellationToken cancellationToken)
    {
        var user = userProvider.RequireAuthorizedUser();

        var deleteNode = await fileNodeRepository.GetNode(user.Id, id, cancellationToken);
        if (deleteNode.NodeType == NodeType.File)
        {
            await MoveFileNodeToTrash(deleteNode, cancellationToken);
        }
        // TODO: add folder delete support
    }

    private async Task MoveFileNodeToTrash(FileNode deleteNode, CancellationToken cancellationToken)
    {
        // If file has already been deleted once and now we delete it again. 
        // Its is going to be moved to trash with increased version.
        // Example:
        //    1. Creates: a.txt - version 1.
        //    2. Updates: a.txt - version 2
        //    3. Moved to trash: version 1 and version 2.
        //    4. Creates: a.txt - version 1
        //    5. Moves to trash: creates trash entry for a.txt with version 3.
        
        // Search if there is already a file with same name from same parent in the trash.
        var trashNodes = await trashRepository.GetFileTrashNodes(deleteNode.UserId, deleteNode.FileSystemPath, cancellationToken).ToList();
        // Return the one list biggest version
        var lastTrashNode = trashNodes.MaxBy(n => n.Version);
        var initialVersion = (lastTrashNode?.Version).GetValueOrDefault(0);

        // Fetch version before delete them.
        var fileVersions = await versionsRepository.GetFileHistory(deleteNode.UserId, deleteNode.Id, SortDirection.Asc, cancellationToken).ToList();
        foreach (var fileVersion in fileVersions)
        {
            // history - copy files from version to trash because they may be used in a file copy
            var fileName = await trashService.CopyFileVersionToTrash(deleteNode.UserId, fileVersion.FileName, cancellationToken);
            var nodeVersion = deleteNode with
            {
                ModifiedAt = fileVersion.CreatedAt,
                Size = fileVersion.Size,
                Version = fileVersion.Version
            };
            await trashRepository.MoveNodeFromVersionToTrash(nodeVersion, fileName, initialVersion, cancellationToken);
        }

        var trashName = await trashService.MoveFileToTrash(deleteNode.UserId, deleteNode.FileSystemPath, cancellationToken);
        await trashRepository.MoveNodeToTrash(deleteNode, trashName, initialVersion, cancellationToken);
    }
}