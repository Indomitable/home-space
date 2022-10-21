namespace HomeSpace.Database.Model;

public enum NodeType
{
    File = 0,
    Folder = 1,
}

public record FileNode(
    long Id,
    long UserId,
    string Title,
    long? ParentId,
    NodeType NodeType,
    string FileSystemPath,
    string MimeType,
    DateTime ModifiedAt,
    long Size,
    int Version
);
