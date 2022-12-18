using System.Data;
using System.Runtime.CompilerServices;
using Microsoft.Extensions.Logging;
using Npgsql;

namespace HomeSpace.Database;

public interface IDbTransaction: IDisposable, IAsyncDisposable
{
    Task<int> ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T?> ExecuteScalar<T>(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
        where T: class;
    IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);

    Task Commit(CancellationToken cancellationToken);
    Task Rollback();
}

internal sealed class DbTransaction : IDbTransaction
{
    private readonly NpgsqlConnection connection;
    private readonly NpgsqlTransaction transaction;
    private readonly ILogger<IDbTransaction> logger;

    public DbTransaction(NpgsqlConnection connection, NpgsqlTransaction transaction, ILogger<IDbTransaction> logger)
    {
        this.connection = connection;
        this.transaction = transaction;
        this.logger = logger;
    }
    
    public async Task<int> ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        return await command.ExecuteNonQueryAsync(cancellationToken);
    }

    public async Task<T?> ExecuteScalar<T>(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        logger.LogDebug("[ExecuteScalar] {sql}. Params: {params}", sql, parameters.Select(p => p.Value));
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        var result = await command.ExecuteScalarAsync(cancellationToken);
        return result is T t ? t : default;
    }

    public async Task<T> QueryOne<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        logger.LogDebug("[QueryOne] {sql}. Params: {params}", sql, parameters.Select(p => p.Value));
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        await using var reader = await command.ExecuteReaderAsync(CommandBehavior.SingleRow, cancellationToken);
        await reader.ReadAsync(cancellationToken);
        return factory(reader);
    }

    public async Task<T?> QueryOptional<T>(string sql, Func<NpgsqlDataReader, T> factory, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
        where T: class
    {
        logger.LogDebug("[QueryOptional] {sql}. Params: {params}", sql, parameters.Select(p => p.Value));
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        await using var reader = await command.ExecuteReaderAsync(CommandBehavior.SingleRow, cancellationToken);
        return await reader.ReadAsync(cancellationToken) ? factory(reader) : null;
    }

    public async IAsyncEnumerable<T> Query<T>(string sql, Func<NpgsqlDataReader, T> factory, [EnumeratorCancellation] CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        logger.LogDebug("[Query] {sql}. Params: {params}", sql, parameters.Select(p => p.Value));
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        await command.PrepareAsync(cancellationToken);
        await using var reader = await command.ExecuteReaderAsync(CommandBehavior.Default, cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            yield return factory(reader);
        }
    }

    public async Task Commit(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            // If Cancel is requested before commit do a rollback.
            await Rollback();
        }
        await transaction.CommitAsync(cancellationToken);
    }
    
    public Task Rollback()
    {
        return transaction.RollbackAsync(CancellationToken.None);
    }
    
    private void AddParameters(NpgsqlCommand command, IEnumerable<NpgsqlParameter> parameters)
    {
        foreach (var parameter in parameters)
        {
            command.Parameters.Add(parameter);
        }
    }

    public void Dispose()
    {
        transaction.Dispose();
        connection.Dispose();
    }

    public async ValueTask DisposeAsync()
    {
        await transaction.DisposeAsync();
        await connection.DisposeAsync();
    }
}
