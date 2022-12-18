using HomeSpace.Database.Configuration;
using Microsoft.Extensions.Logging;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbFactory
{
    Task<IDbTransaction> BeginTransaction();
}

internal sealed class DbFactory
{
    private readonly ILogger<IDbTransaction> logger;
    private readonly string connectionString;

    public DbFactory(DbConfiguration configuration, ILogger<IDbTransaction> logger)
    {
        this.logger = logger;
        var connectionBuilder = new NpgsqlConnectionStringBuilder
        {
            Host = configuration.Host,
            Database = configuration.Database,
            Username = configuration.UserName,
            Password = configuration.Password,
            Pooling = true,
            MinPoolSize = 3,
            MaxPoolSize = 10,
            ApplicationName = "home-space"
        };
        connectionString = connectionBuilder.ConnectionString;
    }

    public async Task<IDbTransaction> BeginTransaction()
    {
        var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        var transaction = await connection.BeginTransactionAsync();
        return new DbTransaction(connection, transaction, logger);
    }
}