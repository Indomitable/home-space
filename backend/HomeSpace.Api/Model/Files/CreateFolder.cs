using System.ComponentModel.DataAnnotations;

namespace HomeSpace.Api.Model.Files;

public record CreateFolderRequest([property: Required]long ParentId, [property: Required] string Name);

public enum CreateFolderResultType
{
    FileWithSameNameExist,
    FolderWithSameNameExist,
    Success
}

public record CreateFolderResult(CreateFolderResultType Type, DisplayFileNode? FileNode);
