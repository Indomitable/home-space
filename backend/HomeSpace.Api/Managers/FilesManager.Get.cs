using System.Net.Mime;
using HomeSpace.Api.Model.Files;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Managers;

public partial class FilesManager
{
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
}