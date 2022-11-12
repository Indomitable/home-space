using HomeSpace.Database.Model;

namespace HomeSpace.Api.Managers;

internal partial class FilesManager
{
    private async Task PermanentDeleteFile(FileNode target, CancellationToken cancellationToken)
    {
        // When delete file delete its versions. We  are not going to delete versions files for now.
        // because if file was copied another file may have reference to them.
        // TODO: implement a background service which cleans orphan versions.
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