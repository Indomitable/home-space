using System.ComponentModel.DataAnnotations;

namespace HomeSpace.Api.Model.Files;

public record CreateFolderRequest([Required]long ParentId, [Required] string Name);

public enum CreateFolderResultType
{
    FileWithSameNameExist,
    FolderWithSameNameExist,
    Success
}

public record CreateFolderResult(CreateFolderResultType Type, FileNodeResponse? FileNode);
