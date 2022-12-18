using HomeSpace.Database;
using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Operations;

public interface ITransactionFactory
{
    Task<ITransaction> BeginTransaction();
}

internal sealed class TransactionFactory : ITransactionFactory
{
    private readonly IDbFactory dbFactory;
    private readonly IFileSystem fileSystem;
    private readonly ILoggerFactory loggerFactory;

    public TransactionFactory(
        IDbFactory dbFactory,
        IFileSystem fileSystem,
        ILoggerFactory loggerFactory)
    {
        this.dbFactory = dbFactory;
        this.fileSystem = fileSystem;
        this.loggerFactory = loggerFactory;
    }

    public async Task<ITransaction> BeginTransaction()
    {
        var dbTransaction = await dbFactory.BeginTransaction();
        return new Transaction(dbTransaction, fileSystem, loggerFactory);
    }
}
