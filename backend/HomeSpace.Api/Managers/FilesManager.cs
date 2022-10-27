using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Services;
using Microsoft.AspNetCore.StaticFiles;

namespace HomeSpace.Api.Managers;

public interface IFilesManager
{
    Task<PagedResult<DisplayFileNode>> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection, CancellationToken cancellationToken);

    IAsyncEnumerable<FileNodeResponse> GetParents(long id, CancellationToken cancellationToken);
    Task<GetFileResult> GetFiles(long[] id, CancellationToken cancellationToken);
    Task<CreateFolderResult> CreateFolder(long parentId, string name);
    Task<UploadFileResult> UploadFile(long parentId, IFormFile file, CancellationToken cancellationToken);
    Task<IReadOnlyList<CopyNodeResult>> CopyNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);
    Task<IReadOnlyList<MoveNodeResult>> MoveNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);
    Task<RenameNodeResult> RenameNode(long id, string name, CancellationToken cancellationToken);
    Task<string> UploadFileChunk(string id, IFormFile file, int chunk, int totalChunks, CancellationToken cancellationToken);
    
    Task<UploadFileResult> UploadLastFileChunk(string id, long requestId, IFormFile file, string fileName, string mimeType,
        long fileSize, int totalChunks, string hash, CancellationToken cancellationToken);
}

internal sealed partial class FilesManager : IFilesManager
{
    private readonly IFileNodeRepository repository;
    private readonly ICurrentUserProvider currentUserProvider;
    private readonly IFilesService filesService;
    private readonly IPathsService pathsService;
    private readonly IVersionsManager versionsManager;
    private readonly IContentTypeProvider contentTypeProvider;
    private readonly ILogger<FilesManager> logger;

    public FilesManager(
        ICurrentUserProvider currentUserProvider,
        IFileNodeRepository repository, 
        IFilesService filesService,
        IPathsService pathsService,
        IVersionsManager versionsManager,
        IContentTypeProvider contentTypeProvider,
        ILogger<FilesManager> logger)
    {
        this.currentUserProvider = currentUserProvider;
        this.repository = repository;
        this.filesService = filesService;
        this.pathsService = pathsService;
        this.versionsManager = versionsManager;
        this.contentTypeProvider = contentTypeProvider;
        this.logger = logger;
    }
}