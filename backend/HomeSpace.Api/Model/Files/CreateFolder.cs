namespace HomeSpace.Api.Model.Files;

public record CreateFolderRequest(long ParentId, string Name);

public enum CreateFolderResultType
{
    FileWithSameNameExist,
    FolderWithSameNameExist,
    Success
}

public record CreateFolderResult(CreateFolderResultType Type, FileNodeResponse? FileNode);
