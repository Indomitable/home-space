using HomeSpace.Database.Model;

namespace HomeSpace.Database.Repository;

public interface IAuthenticationRepository
{
    Task AddAuthentication(UserAuthentication userAuthentication);
    Task<IAuthenticationType?> GetAuthentication(long userId, AuthenticationType type, CancellationToken cancellationToken);
    Task SaveRefreshToken(long userId, string refreshToken, DateTime validTo, CancellationToken cancellationToken);
    Task<User?> GetUserByRefreshToken(string refreshToken, CancellationToken cancellationToken);
    Task DeleteRefreshToken(string refreshToken, CancellationToken cancellationToken);
}

internal class AuthenticationRepository : IAuthenticationRepository
{
    private readonly IDbAccess dbAccess;

    public AuthenticationRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }
    
    public async Task AddAuthentication(UserAuthentication userAuthentication)
    {
        var authId = await userAuthentication.AuthenticationType.Add(dbAccess);
        const string insertSql = "insert into authentication (user_id, auth_type_id, auth_id) values ($1, $2, $3)";
        await dbAccess.ExecuteNonQuery(insertSql,
            CancellationToken.None,
            DbParameter.Create(userAuthentication.UserId),
            DbParameter.Create((short)userAuthentication.Type),
            DbParameter.Create(authId));
    }
    
    public async Task<IAuthenticationType?> GetAuthentication(long userId, AuthenticationType type, CancellationToken cancellationToken)
    {
        const string selectSql = "select auth_id from authentication where user_id = $1 and auth_type_id = $2";
        var authId = await dbAccess.ExecuteScalar<long?>(selectSql,
            cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create((short)type));
        if (!authId.HasValue)
        {
            return null;
        }
        return type switch
        {
            AuthenticationType.Password => await PasswordAuthentication.Create(dbAccess, authId.Value),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };
    }

    public async Task SaveRefreshToken(long userId, string refreshToken, DateTime validTo, CancellationToken cancellationToken)
    {
        const string sql = "insert into refresh_tokens (token, user_id, valid_to) values ($1, $2, $3)";
        await dbAccess.ExecuteNonQuery(sql, cancellationToken,
            DbParameter.Create(refreshToken),
            DbParameter.Create(userId),
            DbParameter.Create(validTo)
        );
    }

    public async Task<User?> GetUserByRefreshToken(string refreshToken, CancellationToken cancellationToken)
    {
        const string sql = @"select u.id, u.name from users u
    inner join refresh_tokens rt on u.id = rt.user_id
    where rt.token = $1 and rt.valid_to > $2";
        var user = await dbAccess.QueryOptional(sql, User.FromReader, cancellationToken,
            DbParameter.Create(refreshToken),
            DbParameter.Create(DateTime.UtcNow)
        );
        return user;
    }

    public async Task DeleteRefreshToken(string refreshToken, CancellationToken cancellationToken)
    {
        const string sql = "delete from refresh_tokens where token = $1";
        await dbAccess.ExecuteNonQuery(sql, cancellationToken, DbParameter.Create(refreshToken));
    }
}