using HomeSpace.Database.Model;

namespace HomeSpace.Database.Repository;

public interface IAuthenticationRepository
{
    Task AddAuthentication(UserAuthentication userAuthentication);
    Task<IAuthenticationType?> GetAuthentication(long userId, AuthenticationType type);
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
            DbParameter.Create(userAuthentication.UserId),
            DbParameter.Create((short)userAuthentication.Type),
            DbParameter.Create(authId));
    }
    
    public async Task<IAuthenticationType?> GetAuthentication(long userId, AuthenticationType type)
    {
        const string selectSql = "select auth_id from authentication where user_id = $1 and auth_type_id = $2";
        var authId = await dbAccess.ExecuteScalar<long?>(selectSql,
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
}