using HomeSpace.Infrastructure.Model;
using Npgsql;

namespace HomeSpace.Database.Model;

public record TrashNode(    
    long Id,
    long UserId,
    string Title,
    long? ParentId,
    NodeType NodeType,
    string FileSystemPath,
    string MimeType,
    DateTime VersionCreatedAt,
    DateTime DeletedAt,
    long Size,
    int Version,
    string FileName
)
{
    public static TrashNode FromReader(NpgsqlDataReader reader)
    {
        return new TrashNode (
            reader.GetFieldValue<long>(0),
            reader.GetFieldValue<long>(1),
            reader.GetFieldValue<string>(2),
            reader.GetFieldValue<long?>(3),
            (NodeType)reader.GetFieldValue<short>(4),
            reader.GetFieldValue<string>(5),
            reader.GetFieldValue<string>(6),
            reader.GetFieldValue<DateTime>(7),
            reader.GetFieldValue<DateTime>(8),
            reader.GetFieldValue<long>(9),
            reader.GetFieldValue<int>(10),
            reader.GetFieldValue<string>(11)
        );
    }
}