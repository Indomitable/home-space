using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Model.Files;

public record GetNodeByPathRequest(string Path);

public record GetFileResult(Stream Content, string Name, string ContentType);

public record FileNodeResponse
{
    public long Id { get; set; }
    public long UserId { get; set; }
    public string Title { get; set; }
    public long? ParentId { get; set; } 
    public NodeType NodeType { get; set; }
    public string FileSystemPath { get; set; }
    public string MimeType { get; set; }
    public DateTime ModifiedAt { get; set; }
    public long Size { get; set; }
    public int Version { get; set; }

    public static FileNodeResponse Map(FileNode fileNode)
    {
        return new FileNodeResponse
        {
            Id = fileNode.Id,
            UserId = fileNode.UserId,
            Title = fileNode.Title,
            ParentId = fileNode.ParentId,
            NodeType = fileNode.NodeType,
            FileSystemPath = fileNode.FileSystemPath,
            MimeType = fileNode.MimeType,
            ModifiedAt = fileNode.ModifiedAt,
            Size = fileNode.Size,
            Version = fileNode.Version,
        };
    }
}