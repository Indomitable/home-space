using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Database.Repository;

public interface IVersionsRepository
{
    IAsyncEnumerable<FileVersion> GetFileHistory(long userId, long id, SortDirection sortDirection, CancellationToken cancellationToken);
    Task AddFileVersion(long userId, long id, int version, long size, string name, byte[]? hashsum);
    Task CopyFileHistory(long sourceUserId, long sourceId, long destinationUserId, long destinationId, CancellationToken cancellationToken);
    Task DeleteFileHistory(long userId, long id, CancellationToken cancellationToken);
}

internal sealed class VersionsRepository : IVersionsRepository
{
    private readonly IDbAccess dbAccess;

    public VersionsRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }

    public IAsyncEnumerable<FileVersion> GetFileHistory(long userId, long id, SortDirection sortDirection, CancellationToken cancellationToken)
    {
        var sql = @"select id, user_id, node_version, created_at, node_size, file_name 
from file_versions
where user_id = $1 and id = $2
order by node_version " + sortDirection.GetOrderByDirection(); 
        // We need to order by node version but to be sure that the current one is last. 
        return dbAccess.Query(sql, FileVersion.FromReader,
            cancellationToken, DbParameter.Create(userId), DbParameter.Create(id));
    }
    
    public Task AddFileVersion(long userId, long id, int version, long size, string name, byte[]? hashsum)
    {
        const string sql = @"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name, hashsum) 
values ($1, $2, $3, $4, $5, $6, $7)";
        return dbAccess.ExecuteNonQuery(sql, CancellationToken.None,
            DbParameter.Create(id),
            DbParameter.Create(userId),
            DbParameter.Create(version),
            DbParameter.Create(DateTime.UtcNow),
            DbParameter.Create(size),
            DbParameter.Create(name),
            DbParameter.Create(hashsum)
        );
    }

    public Task CopyFileHistory(long sourceUserId, long sourceId, long destinationUserId, long destinationId,
        CancellationToken cancellationToken)
    {
        const string sql = @"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name)
        select $4, $3, node_version, created_at, node_size, file_name from file_versions fv 
        where fv.user_id = $1 and fv.id = $2";
        return dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(sourceUserId),
            DbParameter.Create(sourceId),
            DbParameter.Create(destinationUserId),
            DbParameter.Create(destinationId)
        );
    }

    public Task DeleteFileHistory(long userId, long id, CancellationToken cancellationToken)
    {
        const string sql = @"delete file_versions where user_id = $1 and fv.id = $2";
        return dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(id),
            DbParameter.Create(userId)
        );
    }
}