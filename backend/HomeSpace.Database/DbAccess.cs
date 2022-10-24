using System.Data;
using System.Runtime.CompilerServices;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbAccess
{
    Task ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T?> ExecuteScalar<T>(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
        where T: class;
    IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<IDbTransaction> StartTransaction();
}

internal sealed class DbAccess : IDbAccess
{
    private readonly IDbCommandFactory commandFactory;
    
    public DbAccess(IDbCommandFactory commandFactory)
    {
        this.commandFactory = commandFactory;
    }

    public Task<IDbTransaction> StartTransaction()
    {
        return commandFactory.CreateTransaction();
    }

    public async Task ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await command.ExecuteNonQuery(cancellationToken);
    }
    
    public async Task<T?> ExecuteScalar<T>(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        var result = await command.ExecuteScalar(cancellationToken);
        return result is T t ? t : default;
    }

    public async Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await using var reader = await command.ExecuteReader(CommandBehavior.SingleRow, cancellationToken);
        await reader.ReadAsync(cancellationToken);
        return factory(reader);
    }
    
    public async Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
        where T: class
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await using var reader = await command.ExecuteReader(CommandBehavior.SingleRow, cancellationToken);
        return await reader.ReadAsync(cancellationToken) ? factory(reader) : null;
    }
    
    public async IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, [EnumeratorCancellation] CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = await commandFactory.Create(sql);
        command.AddParameters(parameters);
        await command.Prepare(cancellationToken);
        await using var reader = await command.ExecuteReader(CommandBehavior.Default, cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            yield return factory(reader);
        }
    }
}

public static class DbAccessExtensions
{
    public static async Task<List<T>> ToList<T>(this IAsyncEnumerable<T> enumerable)
    {
        var container = new List<T>();
        await foreach (var value in enumerable)
        {
            container.Add(value);
        }
        return container;
    }
}
