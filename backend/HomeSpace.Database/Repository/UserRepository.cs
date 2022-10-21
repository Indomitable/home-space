using HomeSpace.Database.Model;
using Microsoft.Extensions.Logging;
using Npgsql;

namespace HomeSpace.Database.Repository;

public interface IUserRepository
{
    Task<User?> GetById(long userId);
    Task<User?> GetByName(string userName);
    Task<User?> CreateUser(string userName);
}

internal sealed class UserRepository : IUserRepository
{
    private readonly IDbAccess dbAccess;
    private readonly ILogger<UserRepository> logger;

    public UserRepository(IDbAccess dbAccess, ILogger<UserRepository> logger)
    {
        this.dbAccess = dbAccess;
        this.logger = logger;
    }
    
    public async Task<User?> GetById(long userId)
    {
        const string sql = "select id, name from users where id = $1";
        return await dbAccess.QueryOne(sql, User.FromReader, new NpgsqlParameter<long> { Value = userId });
    }

    public async Task<User?> GetByName(string userName)
    {
        const string sql = "select id, name from users where name = $1";
        return await dbAccess.QueryOne(sql, User.FromReader, new NpgsqlParameter<string> { Value = userName });
    }

    public async Task<User?> CreateUser(string userName)
    {
        const string sql = "insert into users (name) values ($1) returning id";
        try
        {
            var userId = await dbAccess.ExecuteScalar<long?>(sql, new NpgsqlParameter<string> { Value = userName });
            if (userId.HasValue)
            {
                return new User { Id = userId.Value, Name = userName };
            }
        }
        catch (PostgresException pe) when (pe.SqlState == "23505")
        {
            // unique_violation -> Same user name
            logger.LogError(pe, "User with same name already exists. [UserName: {userName}]", userName);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Unable to create user! [UserName: {userName}]", userName);
        }
        return null;
    }
}
