using HomeSpace.Database.Model;

namespace HomeSpace.Api.Model.Files;

public enum UploadFileResultType
{
    ParentNotFound,
    FolderWithSameNameExist,
    UploadError,
    Success
}

public record UploadFileResult(UploadFileResultType Type, FileNode? FileNode = null);

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