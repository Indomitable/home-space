using Npgsql;

namespace HomeSpace.Database.Model;

public class User
{
    public long Id { get; set; }
    public string Name { get; set; }

    public static User FromReader(NpgsqlDataReader reader)
    {
        return new User
        {
            Id = reader.GetFieldValue<long>(0),
            Name = reader.GetFieldValue<string>(1)
        };
    }
}