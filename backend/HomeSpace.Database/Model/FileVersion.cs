using HomeSpace.Infrastructure.Model;
using Npgsql;

namespace HomeSpace.Database.Model;

public record FileVersion(
    long Id,
    long UserId,
    int Version,
    DateTime CreatedAt,
    long Size,
    string FileName
)
{
    public static FileVersion FromReader(NpgsqlDataReader reader)
    {
        return new FileVersion (
            reader.GetFieldValue<long>(0),
            reader.GetFieldValue<long>(1),
            reader.GetFieldValue<int>(2),
            reader.GetFieldValue<DateTime>(3),
            reader.GetFieldValue<long>(4),
            reader.GetFieldValue<string>(5)
        );
    }
}