using System.IO.Compression;
using System.Net.Mime;
using System.Runtime.CompilerServices;
using HomeSpace.Api.Model.Files;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Managers;

internal partial class FilesManager
{
    public async Task<PagedResult<DisplayFileNode>> GetNodes(long parentId, int page, int pageSize, FileNodeSort sortColumn,
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

    public async IAsyncEnumerable<FileNodeResponse> GetParents(long id, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        await foreach (var node in repository.GetParentNodes(user.Id, id, cancellationToken))
        {
            yield return FileNodeResponse.Map(node);
        }
    }

    public async Task<GetFileResult> GetNodesContent(long[] ids, CancellationToken cancellationToken)
    {
        var user = currentUserProvider.RequireAuthorizedUser();
        if (ids.Length == 1)
        {
            return await GetFile(user.Id, ids[0], cancellationToken);
        }

        var tempFile = pathsService.GetDownloadsFile(user.Id);
        Stream? writeStream = null;
        try
        {
            writeStream = File.OpenWrite(tempFile);
            using var zipArchive = new ZipArchive(writeStream, ZipArchiveMode.Create, false);
            foreach (var id in ids)
            {
                var result = await GetFile(user.Id, id, cancellationToken);
                var archiveEntry = zipArchive.CreateEntry(result.Name, CompressionLevel.Fastest);
                await using var entryStream = archiveEntry.Open();
                await result.Content.CopyToAsync(entryStream, cancellationToken);
                await result.Content.FlushAsync(cancellationToken);
                result.Content.Close();
                await result.Content.DisposeAsync();
                entryStream.Close();
            }

            await writeStream.FlushAsync(cancellationToken);
        }
        finally
        {
            if (writeStream != null)
            {
                // Close and dispose write handler. Will open read handler to serve.
                writeStream.Close();
                await writeStream.DisposeAsync(); 
            }
        }
        return new GetFileResult(File.OpenRead(tempFile), "archive.zip", MediaTypeNames.Application.Zip);
    }
    
    private async Task<GetFileResult> GetFile(long userId, long id, CancellationToken cancellationToken)
    {
        var fileNode = await repository.GetNode(userId, id, cancellationToken);
        var (stream, title, contentType) = fileNode.NodeType switch
        {
            NodeType.Folder => (filesService.ZipFolder(userId, fileNode.FileSystemPath), string.Concat(fileNode.Title, ".zip"), MediaTypeNames.Application.Zip),
            NodeType.File => (filesService.ReadFile(userId, fileNode.FileSystemPath), fileNode.Title, fileNode.MimeType),
            _ => throw new ArgumentOutOfRangeException()
        };
        return new GetFileResult(stream, title, contentType);
    }
}