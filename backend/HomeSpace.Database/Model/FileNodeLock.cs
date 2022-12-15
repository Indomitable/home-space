using Npgsql;

namespace HomeSpace.Database.Model;

public enum LockType
{
    Copy,
    Move,
    Delete
}

public class FileNodeLock
{
    public long UserId { get; set; }
    public long Id { get; set; }
    public bool WriteLock { get; set; }
    public LockType Type { get; set; }

    public static FileNodeLock FromReader(NpgsqlDataReader reader)
    {
        return new FileNodeLock
        {
            UserId = reader.GetFieldValue<long>(0),
            Id = reader.GetFieldValue<long>(1),
            WriteLock = reader.GetFieldValue<bool>(2),
            Type = (LockType)reader.GetFieldValue<short>(3)
        };
    }
}