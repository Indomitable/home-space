using Npgsql;

namespace HomeSpace.Database.Repository;

public interface IFileNodeRepository
{
    Task CreateRootNode(long userId);
}

internal sealed class FileNodeRepository : IFileNodeRepository
{
    private readonly IDbAccess access;

    public FileNodeRepository(IDbAccess access)
    {
        this.access = access;
    }

    public async Task CreateRootNode(long userId)
    {
        var sequenceSql =
            $"create sequence file_nodes_user_{userId} as bigint increment by 1 minvalue 1 NO MAXVALUE no cycle owned by file_nodes.id";
        
        var insertSql = $@"insert into file_nodes 
            (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version)
            values (nextval('file_nodes_user_{userId}'), $1, 'ROOT', null, 0, '/', 'inode/directory', $2, 0, 1)";

        await access.ExecuteNonQuery(sequenceSql);
        await access.ExecuteNonQuery(insertSql,
            new NpgsqlParameter<long> { Value = userId },
            new NpgsqlParameter<DateTime> { Value = DateTime.UtcNow }
        );
    }
}