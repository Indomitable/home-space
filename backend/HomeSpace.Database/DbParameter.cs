using Npgsql;

namespace HomeSpace.Database;

public static class DbParameter
{
    /// <summary>
    /// Create new positioned parameter
    /// </summary>
    /// <param name="value"></param>
    /// <typeparam name="T"></typeparam>
    /// <returns></returns>
    public static NpgsqlParameter<T> Create<T>(T value)
    {
        return new NpgsqlParameter<T> { Value = value };
    }
}