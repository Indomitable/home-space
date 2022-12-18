using HomeSpace.Files.Services;

namespace HomeSpace.Operations;

public interface IOperationFactory
{
    InitUserFileSystemOperation CreateInitUserFileSystemOperation();
}

public class OperationFactory : IOperationFactory
{
    private readonly IPathsService pathsService;

    public OperationFactory(IPathsService pathsService)
    {
        this.pathsService = pathsService;
    }

    public InitUserFileSystemOperation CreateInitUserFileSystemOperation()
    {
        return new InitUserFileSystemOperation(pathsService);
    }
}
