using HomeSpace.Database.Model;
using Npgsql;

namespace HomeSpace.Database.Repository;

public interface IAuthenticationRepository
{
    Task AddAuthentication(UserAuthentication userAuthentication);
    Task<IAuthenticationType> GetAuthentication(long userId, AuthenticationType type);
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
        await dbAccess.Insert(insertSql,
            new NpgsqlParameter<long> { Value = userAuthentication.UserId },
            new NpgsqlParameter<short> { Value = (short)userAuthentication.Type },
            new NpgsqlParameter<long> { Value = authId });
    }
    
    public async Task<IAuthenticationType> GetAuthentication(long userId, AuthenticationType type)
    {
        const string selectSql = "select auth_id from authentication where user_id = $1 and auth_type_id = $2";
        var auth_id = (long)(await dbAccess.Scalar(selectSql,
            new NpgsqlParameter<long> { Value = userId },
            new NpgsqlParameter<short> { Value = (short)type }) ?? 0);
        switch (type)
        {
            case AuthenticationType.Password:
                return await PasswordAuthentication.Create(dbAccess, auth_id);
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(type), type, null);
        }
    }
}