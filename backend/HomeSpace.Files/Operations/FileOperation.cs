using HomeSpace.Files.Services;
using Microsoft.Extensions.Logging;

namespace HomeSpace.Files.Operations;

public interface IFileOperation
{
    long UserId { get; }

    ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IFileOperation> logger, CancellationToken cancellationToken);

    IRevertFileOperation CreateRevertOperation();
}

/// <summary>
/// Revert File operation it reverts an operation, so doesn't have CreateRevertOperation and do not accept CancellationToken
/// because the revert can be due to the cancelled operation.
/// </summary>
public interface IRevertFileOperation
{
    long UserId { get; }

    ValueTask<bool> Execute(IPathsService pathsService, IFileSystem fileSystem, ILogger<IRevertFileOperation> logger);
}