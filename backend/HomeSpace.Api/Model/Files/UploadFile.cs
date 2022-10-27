namespace HomeSpace.Api.Model.Files;

public enum UploadFileResultType
{
    FolderWithSameNameExist,
    UploadError,
    Success
}

public record UploadFileResult(UploadFileResultType Type, FileNodeResponse? FileNode);

public record UploadFileRequest(long ParentId, IFormFile File);


public record UploadFileChunkRequest(
    string Id,
    IFormFile File,
    int Chunk,
    int TotalChunks);
    
public record UploadLastFileChunkRequest(
    string Id,
    long ParentId,
    IFormFile File,
    string FileName,
    string MimeType,
    long FileSize,
    int TotalChunks,
    string FileHash
);