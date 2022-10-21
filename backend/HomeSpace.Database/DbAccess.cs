using System.Data;
using HomeSpace.Database.Configuration;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbAccess
{
    Task Insert(string sql, params NpgsqlParameter[] parameters);
    Task<object?> Scalar(string sql, params NpgsqlParameter[] parameters);
    Task<T> ReadOne<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters);
}

internal sealed class DbAccess : IDbAccess
{
    private readonly string connectionString;
    
    public DbAccess(DbConfiguration configuration)
    {
        connectionString = $"Host={configuration.Host};Database={configuration.Database};Username={configuration.UserName};Password={configuration.Password};Pooling=True";
    }

    public async Task Insert(string sql, params NpgsqlParameter[] parameters)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        foreach (var parameter in parameters)
        {
            command.Parameters.Add(parameter);
        }
        await command.ExecuteNonQueryAsync();
    }
    
    public async Task<object?> Scalar(string sql, params NpgsqlParameter[] parameters)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        foreach (var parameter in parameters)
        {
            command.Parameters.Add(parameter);
        }
        return await command.ExecuteScalarAsync();
    }

    public async Task<T> ReadOne<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        foreach (var parameter in parameters)
        {
            command.Parameters.Add(parameter);
        }

        await using var reader = command.ExecuteReader(CommandBehavior.SingleRow);
        await reader.ReadAsync();
        return factory(reader);
    }
}