namespace HomeSpace.Api.Model.Files;

public enum CreateFolderResultType
{
    FileWithSameNameExist,
    FolderWithSameNameExist,
    Success
}

public record CreateFolderResult(CreateFolderResultType Type, DisplayFileNode? FileNode);