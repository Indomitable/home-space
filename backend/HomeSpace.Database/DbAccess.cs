using System.Data;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbAccess
{
    Task ExecuteNonQuery(string sql, params NpgsqlParameter[] parameters);
    Task<T?> ExecuteScalar<T>(string sql, params NpgsqlParameter[] parameters);
    Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters);
    Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters)
        where T: class;
    IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters);
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
    
    public async Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters)
        where T: class
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await using var reader = await command.ExecuteReader(CommandBehavior.SingleRow);
        return reader.Read() ? factory(reader) : null;
    }
    
    public async IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await command.Prepare();
        await using var reader = await command.ExecuteReader();
        while (await reader.ReadAsync())
        {
            yield return factory(reader);
        }
    }
}