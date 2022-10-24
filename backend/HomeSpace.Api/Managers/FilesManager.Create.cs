using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Managers;

internal partial class FilesManager
{
    public async Task<CreateFolderResult> CreateFolder(long parentId, string name)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var fileNode = await repository.GetNode(user.Id, parentId, name, CancellationToken.None);
        if (fileNode is not null)
        {
            return fileNode.NodeType == NodeType.Folder
                ? new CreateFolderResult(CreateFolderResultType.FolderWithSameNameExist, null)
                : new CreateFolderResult(CreateFolderResultType.FileWithSameNameExist, null);
        }
        
        var parentNode = await repository.GetNode(user.Id, parentId, CancellationToken.None);
        var node = await CreateFolder(name, parentNode, CancellationToken.None);
        return new CreateFolderResult(CreateFolderResultType.Success, FileNodeResponse.Map(node));
    }

    public async Task<UploadFileResult> UploadFile(long parentId, IFormFile file, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var name = file.FileName;
        var contentType = file.ContentType;
        var fileNode = await repository.GetNode(user.Id, parentId, name, cancellationToken);
        await using var fileStream = file.OpenReadStream();
        if (fileNode is not null)
        {
            if (fileNode.NodeType == NodeType.Folder)
            {
                return new UploadFileResult(UploadFileResultType.FolderWithSameNameExist, null);
            }
            var updatedNode = await OverrideNode(fileStream, contentType, fileNode, cancellationToken);
            return new UploadFileResult(UploadFileResultType.Success, FileNodeResponse.Map(updatedNode));
        }
        var parentNode = await repository.GetNode(user.Id, parentId, cancellationToken);
        var node = await CreateFile(name, fileStream, contentType, parentNode, cancellationToken);
        return new UploadFileResult(UploadFileResultType.Success, FileNodeResponse.Map(node));
    }
    
    private async Task<FileNode> OverrideNode(Stream content, string contentType, FileNode destination, CancellationToken cancellationToken)
    {
        await versionsManager.VersionNode(destination, cancellationToken);
        var size = await filesService.CreateFile(destination.UserId, destination.FileSystemPath, content, cancellationToken);
        await repository.UpdateNode(destination.UserId, destination.Id, size, destination.Version + 1, contentType);
        return destination with
        {
            Version = destination.Version + 1,
            Size = size,
            ModifiedAt = DateTime.UtcNow,
            MimeType = contentType
        };
    }

    private async Task<FileNode> CreateFile(string name, Stream content, string contentType, FileNode parentNode, CancellationToken cancellationToken)
    {
        var nodePath = Path.Join(parentNode.FileSystemPath, name);
        var size = await filesService.CreateFile(parentNode.UserId, nodePath, content, cancellationToken);
        return await repository.CreateNode(parentNode.UserId, parentNode.Id, name, NodeType.File, nodePath, contentType, size);
    }

    private async Task<FileNode> CreateFolder(string name, FileNode parent, CancellationToken cancellationToken)
    {
        var (_, relative) = filesService.CreateFolder(parent.UserId, parent.FileSystemPath, name);
        return await repository.CreateNode(parent.UserId, parent.Id, name, NodeType.Folder, relative, "inode/directory", 0);
    }
}