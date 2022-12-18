using HomeSpace.Database;
using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Operations;

public interface ITransactionFactory
{
    ITransaction BeginTransaction();
}

internal sealed class TransactionFactory : ITransactionFactory
{
    private readonly IDbTransaction dbTransaction;
    private readonly IPathsService pathsService;
    private readonly IFileSystem fileSystem;
    private readonly ILoggerFactory loggerFactory;

    public TransactionFactory(
        IDbTransaction dbTransaction,
        IPathsService pathsService,
        IFileSystem fileSystem,
        ILoggerFactory loggerFactory)
    {
        this.dbTransaction = dbTransaction;
        this.pathsService = pathsService;
        this.fileSystem = fileSystem;
        this.loggerFactory = loggerFactory;
    }

    public ITransaction BeginTransaction()
    {
        return new Transaction(dbTransaction, pathsService, fileSystem, loggerFactory);
    }
}
