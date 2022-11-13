using HomeSpace.Api.Model.Files;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Infrastructure.Model;
using HomeSpace.Security.Model;
using HomeSpace.Security.Services;
using HomeSpace.Services;
using HomeSpace.Services.Factories;
using Microsoft.AspNetCore.StaticFiles;

namespace HomeSpace.Api.Managers;

public interface IFilesManager
{
    /// <summary>
    /// Get list of nodes to display in files view 
    /// </summary>
    Task<PagedResult<DisplayFileNode>> GetNodes(long parentId, int page, int pageSize, FileNodeSort sortColumn,
        SortDirection sortDirection, CancellationToken cancellationToken);

    Task<FileNode?> GetNodeById(long id, CancellationToken cancellationToken);
    
    Task<FileNode?> GetNodeByPath(string path, CancellationToken cancellationToken);

    /// <summary>
    /// Return files or folders content. When multiple ids or ids point to a folder then zip file is returned. 
    /// </summary>
    Task<GetFileResult> GetNodesContent(long[] id, CancellationToken cancellationToken);
    
    /// <summary>
    /// Get node parents. Used to create navigation breadcrumbs
    /// </summary>
    IAsyncEnumerable<FileNode> GetParents(long id, CancellationToken cancellationToken);
    
    /// <summary>
    /// Creates new folder
    /// </summary>
    Task<CreateFolderResult> CreateFolder(long parentId, string name);
    
    /// <summary>
    /// Copy nodes into some location
    /// </summary>
    Task<IReadOnlyDictionary<long, CopyNodeResult>> CopyNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);
    
    /// <summary>
    /// Moves nodes to new location
    /// </summary>
    Task<IReadOnlyDictionary<long, MoveNodeResult>> MoveNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);

    /// <summary>
    /// Rename node
    /// </summary>
    Task<RenameNodeResult> RenameNode(long id, string name, CancellationToken cancellationToken);
    
    /// <summary>
    /// Upload file chunk
    /// </summary>
    Task<string> UploadFileChunk(string id, IFormFile file, int chunk, int totalChunks, CancellationToken cancellationToken);

    /// <summary>
    /// Upload last file chunk
    /// </summary>
    Task<UploadFileResult> UploadLastFileChunk(string id, long requestId, IFormFile file, string fileName, string mimeType,
        long fileSize, int totalChunks, string hash, CancellationToken cancellationToken);
}

internal sealed partial class FilesManager : IFilesManager
{
    private readonly HomeSpaceUser user;
    private readonly IFileNodeRepository repository;
    private readonly IFilesService filesService;
    private readonly IPathsService pathsService;
    private readonly IVersionsManager versionsManager;
    private readonly IContentTypeProvider contentTypeProvider;
    private readonly IJobManager jobManager;
    private readonly ICalcHashSumFactory calcHashSumFactory;
    private readonly ILogger<FilesManager> logger;

    public FilesManager(
        ICurrentUserProvider currentUserProvider,
        IFileNodeRepository repository, 
        IFilesService filesService,
        IPathsService pathsService,
        IVersionsManager versionsManager,
        IContentTypeProvider contentTypeProvider,
        IJobManager jobManager,
        ICalcHashSumFactory calcHashSumFactory,
        ILogger<FilesManager> logger)
    {
        user = currentUserProvider.RequireAuthorizedUser();
        this.repository = repository;
        this.filesService = filesService;
        this.pathsService = pathsService;
        this.versionsManager = versionsManager;
        this.contentTypeProvider = contentTypeProvider;
        this.jobManager = jobManager;
        this.calcHashSumFactory = calcHashSumFactory;
        this.logger = logger;
    }
}