using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Model;

namespace HomeSpace.Api.Managers;

internal partial class FilesManager
{
    public async Task<Dictionary<long, DeleteNodeResult>> MoveNodesToTrash(IReadOnlyCollection<long> nodeIds, CancellationToken cancellationToken)
    {
        var result = new Dictionary<long, DeleteNodeResult>(nodeIds.Count);
        foreach (var chunk in nodeIds.Chunk(CopyChunkSize))
        {
            var chunkResult = await Task.WhenAll(chunk.Select(async id =>
            {
                var deleteResult = await trashManager.MoveToTrash(id, cancellationToken);
                return (id, deleteResult);
            }));
            foreach (var (id, deleteResult) in chunkResult)
            {
                result.Add(id, deleteResult);
            }
        }

        return result;
    }
    
    /// <summary>
    /// Delete file after move:
    /// 1. Delete history associated with the old file.
    /// 2. Delete db entry
    /// 3. Delete actual file.
    /// </summary>
    private async Task DeleteMovedFile(FileNode target, CancellationToken cancellationToken)
    {
        await versionsManager.DeleteHistory(target, cancellationToken);
        await repository.DeleteNode(target.UserId, target.Id, cancellationToken);
        await filesService.DeleteFile(target.UserId, target.FileSystemPath, cancellationToken);
    }
    
    private async Task PermanentDeleteFolder(FileNode target, CancellationToken cancellationToken)
    {
        // TODO: Delete versions too.
        await repository.DeleteNodeRecursive(target.UserId, target.Id, cancellationToken);
        await filesService.DeleteFolder(target.UserId, target.FileSystemPath, cancellationToken);
    }
}