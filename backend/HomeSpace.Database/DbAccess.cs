using System.Data;
using HomeSpace.Database.Configuration;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbAccess
{
    Task ExecuteNonQuery(string sql, params NpgsqlParameter[] parameters);
    Task<T?> ExecuteScalar<T>(string sql, params NpgsqlParameter[] parameters);
    Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters);
}

internal sealed class DbAccess : IDbAccess
{
    private readonly IDbCommandFactory commandFactory;
    
    public DbAccess(IDbCommandFactory commandFactory)
    {
        this.commandFactory = commandFactory;
    }

    public async Task ExecuteNonQuery(string sql, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await command.ExecuteNonQuery();
    }
    
    public async Task<T?> ExecuteScalar<T>(string sql, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        var result = await command.ExecuteScalar();
        return result is T t ? t : default;
    }

    public async Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await using var reader = await command.ExecuteReader(CommandBehavior.SingleRow);
        await reader.ReadAsync();
        return factory(reader);
    }
}