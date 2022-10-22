namespace HomeSpace.Database.Repository;

public interface IVersionsRepository
{
    Task AddFileVersion(long userId, long id, int version, long size, string name);
}

internal sealed class VersionsRepository : IVersionsRepository
{
    private readonly IDbAccess dbAccess;

    public VersionsRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }
    
    public async Task AddFileVersion(long userId, long id, int version, long size, string name)
    {
        const string sql = @"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name) 
values ($1, $2, $3, $4, $5, $6);";
        await dbAccess.ExecuteNonQuery(sql, CancellationToken.None,
            DbParameter.Create(id),
            DbParameter.Create(userId),
            DbParameter.Create(version),
            DbParameter.Create(DateTime.UtcNow),
            DbParameter.Create(size),
            DbParameter.Create(name)
        );
    }
}