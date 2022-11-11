using HomeSpace.Infrastructure.Model;
using Npgsql;

namespace HomeSpace.Database.Model;

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
    int Version,
    byte[]? HashSum
)
{
    public static FileNode FromReader(NpgsqlDataReader reader)
    {
        return new FileNode (
            reader.GetFieldValue<long>(0),
            reader.GetFieldValue<long>(1),
            reader.GetFieldValue<string>(2),
            reader.GetFieldValue<long?>(3),
            (NodeType)reader.GetFieldValue<short>(4),
            reader.GetFieldValue<string>(5),
            reader.GetFieldValue<string>(6),
            reader.GetFieldValue<DateTime>(7),
            reader.GetFieldValue<long>(8),
            reader.GetFieldValue<int>(9),
            reader.IsDBNull(10) ? null : reader.GetFieldValue<byte[]>(10)
        );
    }
}
