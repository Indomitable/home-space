namespace HomeSpace.Database.Repository;

public interface IVersionsRepository
{
    Task AddFileVersion(long userId, long id, int version, long size, string name);
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
    
    public Task AddFileVersion(long userId, long id, int version, long size, string name)
    {
        const string sql = @"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name) 
values ($1, $2, $3, $4, $5, $6)";
        return dbAccess.ExecuteNonQuery(sql, CancellationToken.None,
            DbParameter.Create(id),
            DbParameter.Create(userId),
            DbParameter.Create(version),
            DbParameter.Create(DateTime.UtcNow),
            DbParameter.Create(size),
            DbParameter.Create(name)
        );
    }

    public Task CopyFileHistory(long sourceUserId, long sourceId, long destinationUserId, long destinationId,
        CancellationToken cancellationToken)
    {
        const string sql = @"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name)
        select $5, $4, node_version, created_at, node_size, file_name from file_versions fv 
        where fv.user_id = $1 and fv.id = $2";
        return dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(sourceUserId),
            DbParameter.Create(sourceId),
            DbParameter.Create(destinationUserId)
        );
    }

    public Task DeleteFileHistory(long userId, long id, CancellationToken cancellationToken)
    {
        const string sql = @"delete file_versions where user_id = $1 and fv.id = $2";
        return dbAccess.ExecuteNonQuery(sql, CancellationToken.None,
            DbParameter.Create(id),
            DbParameter.Create(userId)
        );
    }
}