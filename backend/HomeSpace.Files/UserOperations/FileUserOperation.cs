using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.UserOperations;

public interface IFileUserOperation
{
    long UserId { get; }

    ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileUserOperation> logger, CancellationToken cancellationToken);

    IRevertFileUserOperation CreateRevertOperation();
}

/// <summary>
/// Revert File operation it reverts an operation, so doesn't have CreateRevertOperation and do not accept CancellationToken
/// because the revert can be due to the cancelled operation.
/// </summary>
public interface IRevertFileUserOperation
{
    long UserId { get; }

    ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileUserOperation> logger);
}
