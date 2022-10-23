using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Managers;

public partial class FilesManager
{
    private static readonly int CopyChunkSize = 3;
    
    public async Task<IReadOnlyList<CopyNodeResult>> CopyNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId,
        CancellationToken cancellationToken)
    {
        var results = new List<CopyNodeResult>(sourceIds.Count);
        foreach (var chunk in sourceIds.Chunk(CopyChunkSize))
        {
            var chunkResults = await Task.WhenAll(chunk.Select(id => CopyNode(id, destinationParentId, cancellationToken)));
            results.AddRange(chunkResults);
        }
        return results;
    }

    public async Task<IReadOnlyList<MoveNodeResult>> MoveNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId,
        CancellationToken cancellationToken)
    {
        var results = new List<MoveNodeResult>(sourceIds.Count);
        foreach (var chunk in sourceIds.Chunk(CopyChunkSize))
        {
            var chunkResults = await Task.WhenAll(chunk.Select(id => MoveNode(id, destinationParentId, cancellationToken)));
            results.AddRange(chunkResults);
        }
        return results;
    }
    
    private async Task<CopyNodeResult> CopyNode(long sourceId, long destinationParentId, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var sourceNode = await repository.GetNode(user.Id, sourceId, cancellationToken);
        var destinationParentNode = await repository.GetNode(user.Id, destinationParentId, cancellationToken);
        var result = sourceNode.NodeType switch
        {
            NodeType.Folder => await CopyFolderNodeRecursive(sourceNode, destinationParentNode, cancellationToken),
            NodeType.File => await CopyFileNode(sourceNode, destinationParentNode, cancellationToken),
            _ => throw new ArgumentOutOfRangeException()
        };
        return new CopyNodeResult(result.Type, result.Node is null ? null : FileNodeResponse.Map(result.Node));
    }
    
    private async Task<MoveNodeResult> MoveNode(long sourceId, long destinationParentId, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var sourceNode = await repository.GetNode(user.Id, sourceId, cancellationToken);
        var destinationParentNode = await repository.GetNode(user.Id, destinationParentId, cancellationToken);
        var copyResult = sourceNode.NodeType switch
        {
            NodeType.Folder => await CopyFolderNodeRecursive(sourceNode, destinationParentNode, cancellationToken),
            NodeType.File => await CopyFileNode(sourceNode, destinationParentNode, cancellationToken),
            _ => throw new ArgumentOutOfRangeException()
        };
        if (copyResult.Type == CopyNodeResultType.Success)
        {
            var deleteResult = sourceNode.NodeType switch
            {
                NodeType.Folder => PermanentDeleteFolder(sourceNode, cancellationToken),
                NodeType.File => PermanentDeleteFile(sourceNode, cancellationToken),
                _ => throw new ArgumentOutOfRangeException()
            };
            
        }
        return new MoveNodeResult(copyResult.Type, false, null);
    }
    
    public async Task<RenameNodeResult> RenameNode(long id, string name, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var sourceNode = await repository.GetNode(user.Id, id, cancellationToken);
        var sameNode = await repository.GetNode(user.Id, sourceNode.ParentId.GetValueOrDefault(0), name, cancellationToken);
        if (sameNode is not null)
        {
            return new RenameNodeResult(RenameNodeResultType.NodeWithSameNameExist, null);
        }
        var destination = await filesService.RenameFile(user.Id, sourceNode.FileSystemPath, name, cancellationToken);
        await repository.RenameNode(user.Id, id, name, destination, cancellationToken);
        var node = sourceNode with
        {
            Title = name,
            FileSystemPath = destination
        };
        return new RenameNodeResult(RenameNodeResultType.NodeWithSameNameExist, FileNodeResponse.Map(node));
    }
    
    public record CopyNodeResultInner(CopyNodeResultType Type, FileNode? Node);
    
    private async Task<CopyNodeResultInner> CopyFileNode(FileNode source, FileNode destinationParent, CancellationToken cancellationToken)
    {
        // Find if in the destination has the same node.
        var sameNode = await repository.GetNode(destinationParent.UserId, destinationParent.Id, source.Title,
            cancellationToken);
        if (sameNode is not null)
        {
            if (sameNode.NodeType == NodeType.Folder)
            {
                return new CopyNodeResultInner(CopyNodeResultType.CopyFileOverFolderError, null);
            }

            await using var content = filesService.ReadFile(source.UserId, source.FileSystemPath);
            var node = await OverrideNode(content, source.MimeType, sameNode, cancellationToken);
            return new CopyNodeResultInner(CopyNodeResultType.Success, node);
        }
        else
        {
            await using var content = filesService.ReadFile(source.UserId, source.FileSystemPath);
            var node = await CreateFile(source.Title, content, source.MimeType, destinationParent,
                cancellationToken);
            // When we copy node from one place to another and if it has versions
            // copy them too in order to keep history. 
            await versionsManager.CopyHistory(source, node, cancellationToken);
            return new CopyNodeResultInner(CopyNodeResultType.Success, node);
        }
    }
    
    private async Task<CopyNodeResultInner> CopyFolderNode(FileNode source, FileNode destinationParent, CancellationToken cancellationToken)
    {
        var sameNode = await repository.GetNode(destinationParent.UserId, destinationParent.Id, source.Title,
            cancellationToken);
        if (sameNode is not null)
        {
            if (sameNode.NodeType == NodeType.File)
            {
                return new CopyNodeResultInner(CopyNodeResultType.CopyFolderOverFileError, null);
            }
            return new CopyNodeResultInner(CopyNodeResultType.Success, sameNode);
        }
        var node = await CreateFolder(source.Title, destinationParent, cancellationToken);
        return new CopyNodeResultInner(CopyNodeResultType.Success, node);
    }

    private async Task<CopyNodeResultInner> CopyFolderNodeRecursive(FileNode source, FileNode destinationParent, CancellationToken cancellationToken)
    {
        // Create parent folder in the destination
        var copyParentResult = await CopyFolderNode(source, destinationParent, cancellationToken);
        if (copyParentResult is { Type: CopyNodeResultType.Success, Node: { } currentParent })
        {
            await foreach (var childNode in
                           repository.GetChildNodes(source.UserId, destinationParent.Id, cancellationToken))
            {
                switch (childNode.NodeType)
                {
                    case NodeType.Folder:
                        await CopyFolderNodeRecursive(childNode, currentParent, cancellationToken);
                        break;
                    case NodeType.File:
                        await CopyFileNode(childNode, destinationParent, cancellationToken);
                        break;
                    default:
                        throw new ArgumentOutOfRangeException();
                }
            }
        }
        return copyParentResult;
    }
    
}