using HomeSpace.Database.Model;
using Npgsql;
using NpgsqlTypes;

namespace HomeSpace.Database.Repository;

public class PasswordAuthentication: IAuthenticationType
{
    public byte[] Hash { get; }
    public byte[] Salt { get; }

    public PasswordAuthentication(byte[] hash, byte[] salt)
    {
        Hash = hash;
        Salt = salt;
    }
    
    public async Task<long> Add(IDbAccess dbAccess)
    {
        const string sql = "insert into authentication_password (hash, salt) values ($1, $2) RETURNING id";
        var authId = await dbAccess.Scalar(sql,
            new NpgsqlParameter<byte[]> { NpgsqlDbType = NpgsqlDbType.Bytea, Value = Hash },
            new NpgsqlParameter<byte[]> { NpgsqlDbType = NpgsqlDbType.Bytea, Value = Salt });
        return (long)(authId ?? 0);
    }

    public static async Task<IAuthenticationType> Create(IDbAccess dbAccess, long authId)
    {
        const string sql = "select hash, salt from authentication_password where id = $1";
        return await dbAccess.ReadOne(sql, reader =>
        {
            var hash = reader.GetFieldValue<byte[]>(0);
            var salt = reader.GetFieldValue<byte[]>(1);
            return new PasswordAuthentication(hash, salt);
        }, new NpgsqlParameter { Value = authId });
    }
}