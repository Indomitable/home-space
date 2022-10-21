using System.Data;
using HomeSpace.Database.Configuration;
using Npgsql;

namespace HomeSpace.Database;

internal interface IDbCommandFactory
{
    Task<DbCommand> Create(string sql);
}

internal sealed class DbCommandFactory : IDbCommandFactory
{
    private readonly string connectionString;

    public DbCommandFactory(DbConfiguration configuration)
    {
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

    public async Task<DbCommand> Create(string sql)
    {
        var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        var command = connection.CreateCommand();
        command.CommandText = sql;
        return new DbCommand(connection, command);
    }
}

internal class DbCommand : IDisposable, IAsyncDisposable
{
    private readonly NpgsqlConnection connection;
    private readonly NpgsqlCommand command;

    public DbCommand(NpgsqlConnection connection, NpgsqlCommand command)
    {
        this.connection = connection;
        this.command = command;
    }

    public void AddParameters(IEnumerable<NpgsqlParameter> parameters)
    {
        foreach (var parameter in parameters)
        {
            command.Parameters.Add(parameter);
        }
    }
    
    public Task<int> ExecuteNonQuery()
    {
        return command.ExecuteNonQueryAsync();
    }
    
    public Task<NpgsqlDataReader> ExecuteReader(CommandBehavior commandBehavior = CommandBehavior.Default)
    {
        return command.ExecuteReaderAsync(commandBehavior);
    }
    
    public Task<object?> ExecuteScalar()
    {
        return command.ExecuteScalarAsync();
    }

    public void Dispose()
    {
        command.Dispose();
        connection.Close();
        connection.Dispose();
    }

    public async ValueTask DisposeAsync()
    {
        await command.DisposeAsync();
        await connection.CloseAsync();
        await connection.DisposeAsync();
    }
}