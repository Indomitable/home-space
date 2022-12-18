using HomeSpace.Files.FileOperations;
using HomeSpace.Files.Services;

namespace HomeSpace.Operations;

public class InitUserFileSystemOperation
{
    private readonly IPathsService pathsService;

    public InitUserFileSystemOperation(IPathsService pathsService)
    {
        this.pathsService = pathsService;
    }

    public async Task Execute(ITransaction transaction, long userId, CancellationToken cancellationToken)
    {
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserDirectory(userId)), cancellationToken);
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserSystemDirectory(userId)), cancellationToken);
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserTrashDirectory(userId)), cancellationToken);
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserVersionsDirectory(userId)), cancellationToken);
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserDownloadsDirectory(userId)), cancellationToken);
        await transaction.ExecuteFileOperation(new CreateFolder(pathsService.UserUploadsDirectory(userId)), cancellationToken);
    }
}
