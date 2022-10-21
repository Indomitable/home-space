using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Repository;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Services;
using SystemFileManager = HomeSpace.Files.Services.IFilesManager;

namespace HomeSpace.Api.Managers;

public interface IFilesManager
{
    Task<PagedResult<DisplayFileNode>> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection = SortDirection.Asc);

    Task<GetFileResult> GetFile(long id);
    Task<CreateFolderResult> CreateFolder(long parentId, string name);
}

sealed class FilesManager : IFilesManager
{
    private readonly IFileNodeRepository repository;
    private readonly ICurrentUserProvider currentUserProvider;
    private readonly SystemFileManager filesManager;

    public FilesManager(IFileNodeRepository repository, 
        ICurrentUserProvider currentUserProvider, 
        SystemFileManager filesManager)
    {
        this.repository = repository;
        this.currentUserProvider = currentUserProvider;
        this.filesManager = filesManager;
    }
    
    public async Task<PagedResult<DisplayFileNode>> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection = SortDirection.Asc)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var sortField = ResolveSortColumn(sortColumn);
        var files = await repository.GetNodes(user.Id, parentId, page, pageSize, sortField, sortDirection);
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

    public async Task<GetFileResult> GetFile(long id)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var fileNode = await repository.GetNode(user.Id, id);
        var (stream, title) = fileNode.NodeType switch
        {
            NodeType.Folder => (filesManager.ZipFolder(user.Id, fileNode.FileSystemPath), string.Concat(fileNode.Title, ".zip")),
            NodeType.File => (filesManager.ReadFile(user.Id, fileNode.FileSystemPath), fileNode.Title),
            _ => throw new ArgumentOutOfRangeException()
        };
        return new GetFileResult(stream, title);
    }

    public async Task<CreateFolderResult> CreateFolder(long parentId, string name)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        var fileNode = await repository.GetNode(user.Id, parentId, name);
        if (fileNode is not null)
        {
            return fileNode.NodeType == NodeType.Folder
                ? new CreateFolderResult(CreateFolderResultType.FolderWithSameNameExist, null)
                : new CreateFolderResult(CreateFolderResultType.FileWithSameNameExist, null);
        }
        
        var parentNode = await repository.GetNode(user.Id, parentId);
        var (_, relative) = filesManager.CreateFolder(user.Id, parentNode.FileSystemPath, name);
        var node = await repository.CreateNode(user.Id, parentId, name, NodeType.Folder, relative, "inode/directory", 0);
        return new CreateFolderResult(CreateFolderResultType.Success, DisplayFileNode.Map(node, false));
    }
}