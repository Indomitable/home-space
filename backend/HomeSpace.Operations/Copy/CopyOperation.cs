using HomeSpace.Database;
using HomeSpace.Database.Model;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Operations;
using HomeSpace.Files.Services;

namespace HomeSpace.Operations.Copy;

public class CopyOperation
{
    private readonly IDbFactory dbFactory;
    private readonly IPathsService pathsService;
    private readonly IFileLocksRepository fileLocksRepository;
    private readonly IFileNodeRepository fileNodeRepository;
    private readonly IVersionsRepository versionsRepository;

    public CopyOperation(IDbFactory dbFactory,
        IPathsService pathsService,
        IFileLocksRepository fileLocksRepository,
        IFileNodeRepository fileNodeRepository,
        IVersionsRepository versionsRepository)
    {
        this.dbFactory = dbFactory;
        this.pathsService = pathsService;
        this.fileLocksRepository = fileLocksRepository;
        this.fileNodeRepository = fileNodeRepository;
        this.versionsRepository = versionsRepository;
    }
    
    public async Task Execute(long userId, IReadOnlyCollection<long> sourceIds, long destinationParentId, CancellationToken cancellationToken)
    {
        var transaction =  await dbFactory.BeginTransaction();
        try
        {
            foreach (var id in sourceIds)
            {
                var copyOperations = new List<IFileOperation>();
                var sourceNode = await fileNodeRepository.GetNode(userId, id, cancellationToken);
                if (sourceNode is null)
                {
                    continue;
                }
                
                var locks = await fileLocksRepository.GetLocks(userId, id, cancellationToken);
                if (locks.Any())
                {
                    // One of parent has lock
                    continue; 
                }
                await fileLocksRepository.AddLock(transaction, userId, id, true, LockType.Copy, cancellationToken);

                var existingFile = await fileNodeRepository.GetNode(userId, destinationParentId, sourceNode.Title, cancellationToken);
                if (existingFile is not null)
                {
                    // there is existing file there so add first create file version operation
                    var versionFile = Guid.NewGuid().ToString("N");
                    await versionsRepository.AddFileVersion(transaction, userId, existingFile.Id, versionFile);
                    copyOperations.Add(new CreateFileVersion(userId, existingFile.FileSystemPath, versionFile));
                }

            }
            
            await transaction.Commit(cancellationToken);
        }
        catch (Exception e)
        {
            await transaction.Rollback();
        }
    }
}
