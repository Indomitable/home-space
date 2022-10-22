namespace HomeSpace.Api.Model.Files;

public enum UploadFileResultType
{
    FolderWithSameNameExist,
    Success
}

public record UploadFileResult(UploadFileResultType Type, DisplayFileNode? FileNode);