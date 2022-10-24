namespace HomeSpace.Database.Repository;

public interface IFavoritesRepository
{
    Task SetFavorite(long userId, long nodeId, CancellationToken cancellationToken);
    Task UnsetFavorite(long userId, long nodeId, CancellationToken cancellationToken);
}

internal sealed class FavoritesRepository : IFavoritesRepository
{
    private readonly IDbAccess dbAccess;

    public FavoritesRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }

    public Task SetFavorite(long userId, long nodeId, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO favorite_nodes (id, user_id) VALUES($2, $1)";
        return dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create(nodeId)
        );
    }
    
    public Task UnsetFavorite(long userId, long nodeId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM favorite_nodes where user_id = $1 and id = $2";
        return dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create(nodeId)
        );
    }
}