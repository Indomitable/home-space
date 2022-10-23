namespace HomeSpace.Api.Model.Files;

public enum UploadFileResultType
{
    FolderWithSameNameExist,
    Success
}

public record UploadFileResult(UploadFileResultType Type, FileNodeResponse? FileNode);

public record UploadFileRequest(long ParentId, IFormFile File);
