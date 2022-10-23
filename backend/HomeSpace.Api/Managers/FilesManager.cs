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
    Task<IReadOnlyList<CopyNodeResult>> CopyNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);
    Task<IReadOnlyList<MoveNodeResult>> MoveNodes(IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken);
    Task<RenameNodeResult> RenameNode(long id, string name, CancellationToken cancellationToken);
}

sealed partial class FilesManager : IFilesManager
{
    private readonly IFileNodeRepository repository;
    private readonly ICurrentUserProvider currentUserProvider;
    private readonly IFilesService filesService;
    private readonly IVersionsManager versionsManager;

    public FilesManager(
        ICurrentUserProvider currentUserProvider,
        IFileNodeRepository repository, 
        IFilesService filesService,
        IVersionsManager versionsManager)
    {
        this.currentUserProvider = currentUserProvider;
        this.repository = repository;
        this.filesService = filesService;
        this.versionsManager = versionsManager;
    }
}