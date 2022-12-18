using HomeSpace.Database;
using HomeSpace.Files.Services;
using HomeSpace.Files.UserOperations;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Operations;

public interface ITransaction
{
    IDbTransaction DbTransaction { get; }
    Task ExecuteFileOperation<T>(T fileOperation, CancellationToken cancellationToken)
        where T: IFileOperation;
    Task Commit(CancellationToken cancellationToken);
    Task RollBack();
}

public class Transaction : ITransaction
{
    private readonly IPathsService pathsService;
    private readonly IFileSystem fileSystem;
    private readonly ILoggerFactory loggerFactory;
    private readonly Stack<IFileOperation> fileOperations = new();

    public Transaction(
        IDbTransaction dbTransaction,
        IPathsService pathsService,
        IFileSystem fileSystem,
        ILoggerFactory loggerFactory)
    {
        this.pathsService = pathsService;
        this.fileSystem = fileSystem;
        this.loggerFactory = loggerFactory;
        DbTransaction = dbTransaction;
    }

    public IDbTransaction DbTransaction { get; }

    public async Task ExecuteFileOperation<T>(T fileOperation, CancellationToken cancellationToken)
        where T: IFileOperation
    {
        var logger = (ILogger<IFileOperation>)loggerFactory.CreateLogger<T>();
        var result = await fileOperation.Execute(pathsService, fileSystem, logger, cancellationToken);
        if (result)
        {
            fileOperations.Push(fileOperation);
        }
        else
        {
            await RollBack();
        }
    }

    public async Task Commit(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            await RollBack();
        }
        await DbTransaction.Commit(CancellationToken.None);
    }

    public async Task RollBack()
    {
        while (fileOperations.Count > 0)
        {
            var operation = fileOperations.Pop();
            var reverseOperation = operation.CreateRevertOperation();
            var logger = loggerFactory.CreateLogger<IRevertFileOperation>();
            await reverseOperation.Execute(pathsService, fileSystem, logger);
        }
        await DbTransaction.Rollback();
    }
}
