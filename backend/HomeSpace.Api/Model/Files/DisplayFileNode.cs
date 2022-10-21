using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Api.Model.Files;

public sealed record DisplayFileNode
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
    public bool IsFavorite { get; set; }

    public static DisplayFileNode Map(FileNode fileNode, bool isFavorite)
    {
        return new DisplayFileNode
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
            IsFavorite = isFavorite
        };
    }
}