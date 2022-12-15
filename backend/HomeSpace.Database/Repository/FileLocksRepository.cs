using HomeSpace.Database.Model;

namespace HomeSpace.Database.Repository;

public interface IFileLocksRepository
{
    Task<IReadOnlyList<FileNodeLock>> GetLocks(long userId, long id, CancellationToken cancellationToken);
    Task AddLock(IDbTransaction transaction, long userId, long id, bool writeLock, LockType lockType, CancellationToken cancellationToken);
    Task RemoveLock(IDbTransaction transaction, long userId, long id, CancellationToken cancellationToken);
}

public class FileLocksRepository : IFileLocksRepository
{
    private readonly IDbAccess dbAccess;

    public FileLocksRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }

    public async Task<IReadOnlyList<FileNodeLock>> GetLocks(long userId, long id, CancellationToken cancellationToken)
    {
        const string sql = @"WITH RECURSIVE query AS (
            select *, 0 as lvl from file_nodes
            where user_id = $1 and id = $2
            UNION ALL
            select n.*, lvl-1 as lev from file_nodes n
            INNER JOIN query p ON p.parent_id = n.id and p.user_id = n.user_id
        )
        select l.user_id, l.id, l.write_lock, l.lock_type from file_node_locks l 
        where exists(select 1 from query q where q.id = l.id and q.user_id = l.user_id)";
        var result = new List<FileNodeLock>();
        await foreach (var node in dbAccess.Query(sql, FileNodeLock.FromReader, cancellationToken,
                           DbParameter.Create(userId), DbParameter.Create(id)))
        {
            result.Add(node);
        }
        return result;
    }

    public async Task AddLock(IDbTransaction transaction, long userId, long id, bool writeLock, LockType lockType, CancellationToken cancellationToken)
    {
        const string sql = @"insert into file_node_locks (user_id, id, write_lock, lock_type) values ($1, $2, $3, $4)";
        await transaction.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create(id),
            DbParameter.Create(writeLock),
            DbParameter.Create((int)lockType)
        );
    }
    
    public async Task RemoveLock(IDbTransaction transaction, long userId, long id, CancellationToken cancellationToken)
    {
        const string sql = @"delete from file_node_locks where user_id = $1 and id = $2";
        await transaction.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create(id)
        );
    }
    
}