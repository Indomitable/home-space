using HomeSpace.Database.Model;

namespace HomeSpace.Api.Model.Files;

public record CreateFolderRequest(long ParentId, string Name);

public enum CreateFolderResultType
{
    ParentNodeNotFound,
    FileWithSameNameExist,
    FolderWithSameNameExist,
    Success
}

public record CreateFolderResult(CreateFolderResultType Type, FileNode? FileNode = null);
