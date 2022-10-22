using System.Data.Common;
using HomeSpace.Database.Model;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Database.Repository;

public interface IUserRepository
{
    Task<User?> GetById(long userId, CancellationToken cancellationToken);
    Task<User?> GetByName(string userName, CancellationToken cancellationToken);
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
    
    public async Task<User?> GetById(long userId, CancellationToken cancellationToken)
    {
        const string sql = "select id, name from users where id = $1";
        return await dbAccess.QueryOne(sql, User.FromReader, cancellationToken, DbParameter.Create(userId));
    }

    public async Task<User?> GetByName(string userName, CancellationToken cancellationToken)
    {
        const string sql = "select id, name from users where name = $1";
        return await dbAccess.QueryOne(sql, User.FromReader, cancellationToken, DbParameter.Create(userName));
    }

    public async Task<User?> CreateUser(string userName)
    {
        const string sql = "insert into users (name) values ($1) returning id";
        try
        {
            var userId = await dbAccess.ExecuteScalar<long?>(sql, CancellationToken.None, DbParameter.Create(userName));
            if (userId.HasValue)
            {
                return new User { Id = userId.Value, Name = userName };
            }
        }
        catch (DbException pe) when (pe.SqlState == "23505")
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
