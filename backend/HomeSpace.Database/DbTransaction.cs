using Npgsql;

namespace HomeSpace.Database;

public interface IDbTransaction: IDisposable, IAsyncDisposable
{
    Task<int> ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters);
    Task Commit(CancellationToken cancellationToken);
    Task Rollback(CancellationToken cancellationToken);
}

public class DbTransaction : IDbTransaction
{
    private readonly NpgsqlConnection connection;
    private readonly NpgsqlTransaction transaction;

    public DbTransaction(NpgsqlConnection connection, NpgsqlTransaction transaction)
    {
        this.connection = connection;
        this.transaction = transaction;
    }
    
    public async Task<int> ExecuteNonQuery(string sql, CancellationToken cancellationToken, params NpgsqlParameter[] parameters)
    {
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        AddParameters(command, parameters);
        return await command.ExecuteNonQueryAsync(cancellationToken);
    }


    public async Task Commit(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            // If Cancel is requested before commit do a rollback.
            await Rollback(CancellationToken.None);
        }
        await transaction.CommitAsync(cancellationToken);
    }
    
    public Task Rollback(CancellationToken cancellationToken)
    {
        return transaction.RollbackAsync(cancellationToken);
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