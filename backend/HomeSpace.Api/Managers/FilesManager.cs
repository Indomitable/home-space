using System.Net.Mime;
using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Services;

namespace HomeSpace.Api.Managers;

public interface IFilesManager
{
    Task<PagedResult<DisplayFileNode>> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection, CancellationToken cancellationToken);

    Task<GetFileResult> GetFile(long id, CancellationToken cancellationToken);
    Task<CreateFolderResult> CreateFolder(long parentId, string name);
    Task<UploadFileResult> UploadFile(long parentId, IFormFile file, CancellationToken cancellationToken);
}

sealed class FilesManager : IFilesManager
{
    private readonly IFileNodeRepository repository;
    private readonly ICurrentUserProvider currentUserProvider;
    private readonly IFilesService filesService;
    private readonly IVersionsManager versionsManager;

    public FilesManager(
        IFileNodeRepository repository, 
        ICurrentUserProvider currentUserProvider,
        IFilesService filesService,
        IVersionsManager versionsManager)
    {
        this.repository = repository;
        this.currentUserProvider = currentUserProvider;
        this.filesService = filesService;
        this.versionsManager = versionsManager;
    }
    
    public async Task<PagedResult<DisplayFileNode>> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var sortField = ResolveSortColumn(sortColumn);
        var files = await repository.GetNodes(user.Id, parentId, page, pageSize, sortField, sortDirection, cancellationToken);
        return files.Map(fn => DisplayFileNode.Map(fn.FileNode, fn.IsFavorite));
    }

    private string ResolveSortColumn(FileNodeSort sort)
    {
        switch (sort)
        {
            case FileNodeSort.Title:
                return "title";
            case FileNodeSort.MimeType:
                return "mime_type";
            case FileNodeSort.ModifiedAt:
                return "modified_at";
            case FileNodeSort.Size:
                return "node_size";
            case FileNodeSort.Favorite:
                return "is_favorite";
            default:
                throw new NotSupportedException($"Sort column {sort} is not supported");
        }
    }

    public async Task<GetFileResult> GetFile(long id, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var fileNode = await repository.GetNode(user.Id, id, cancellationToken);
        var (stream, title, contentType) = fileNode.NodeType switch
        {
            NodeType.Folder => (filesService.ZipFolder(user.Id, fileNode.FileSystemPath), string.Concat(fileNode.Title, ".zip"), MediaTypeNames.Application.Zip),
            NodeType.File => (filesService.ReadFile(user.Id, fileNode.FileSystemPath), fileNode.Title, fileNode.MimeType),
            _ => throw new ArgumentOutOfRangeException()
        };
        return new GetFileResult(stream, title, contentType);
    }

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
        var (_, relative) = filesService.CreateFolder(user.Id, parentNode.FileSystemPath, name);
        var node = await repository.CreateNode(user.Id, parentId, name, NodeType.Folder, relative, "inode/directory", 0);
        return new CreateFolderResult(CreateFolderResultType.Success, DisplayFileNode.Map(node, false));
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
            await versionsManager.VersionNode(fileNode, cancellationToken);
            var size = await filesService.CreateFile(user.Id, fileNode.FileSystemPath, fileStream, cancellationToken);
            await repository.UpdateNode(user.Id, fileNode.Id, size, fileNode.Version + 1, contentType);
            var node = new DisplayFileNode
            {
                Id = fileNode.Id,
                UserId = fileNode.UserId,
                Title = fileNode.Title,
                ParentId = fileNode.ParentId,
                NodeType = fileNode.NodeType,
                FileSystemPath = fileNode.FileSystemPath,
                MimeType = contentType,
                ModifiedAt = DateTime.UtcNow,
                Size = size,
                Version = fileNode.Version + 1,
                IsFavorite = false, // we don't know and don't care for now.
            };
            return new UploadFileResult(UploadFileResultType.Success, node);
        }
        else
        {
            var parentNode = await repository.GetNode(user.Id, parentId, cancellationToken);
            var nodePath = Path.Join(parentNode.FileSystemPath, name);
            var size = await filesService.CreateFile(user.Id, nodePath, fileStream, cancellationToken);
            var node = await repository.CreateNode(user.Id, parentId, name, NodeType.File, nodePath, contentType, size);
            return new UploadFileResult(UploadFileResultType.Success, DisplayFileNode.Map(node, false));
        }
    }
}