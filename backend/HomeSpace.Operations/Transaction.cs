using HomeSpace.Database;
using HomeSpace.Files.FileOperations;
using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Operations;

public interface ITransaction: IDisposable
{
    IDbTransaction DbTransaction { get; }
    Task ExecuteFileOperation<T>(T fileOperation, CancellationToken cancellationToken)
        where T: IFileOperation;
    Task Commit(CancellationToken cancellationToken);
    Task Rollback();
}

public class Transaction : ITransaction
{
    private readonly IFileSystem fileSystem;
    private readonly ILoggerFactory loggerFactory;
    private readonly Stack<IFileOperation> fileOperations = new();

    public Transaction(
        IDbTransaction dbTransaction,
        IFileSystem fileSystem,
        ILoggerFactory loggerFactory)
    {
        this.fileSystem = fileSystem;
        this.loggerFactory = loggerFactory;
        DbTransaction = dbTransaction;
    }

    public IDbTransaction DbTransaction { get; }

    public async Task ExecuteFileOperation<T>(T fileOperation, CancellationToken cancellationToken)
        where T: IFileOperation
    {
        var logger = (ILogger<IFileOperation>)loggerFactory.CreateLogger<T>();
        await fileOperation.Execute(fileSystem, logger, cancellationToken);
        fileOperations.Push(fileOperation);
    }

    public async Task Commit(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            await Rollback();
        }
        await DbTransaction.Commit(CancellationToken.None);
    }

    public async Task Rollback()
    {
        while (fileOperations.Count > 0)
        {
            var operation = fileOperations.Pop();
            var reverseOperation = operation.CreateRevertOperation();
            var logger = loggerFactory.CreateLogger<IRevertFileOperation>();
            await reverseOperation.Execute(fileSystem, logger);
        }
        await DbTransaction.Rollback();
    }

    public void Dispose()
    {
        DbTransaction.Dispose();
    }
}
