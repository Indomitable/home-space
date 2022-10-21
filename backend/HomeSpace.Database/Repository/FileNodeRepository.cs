using HomeSpace.Database.Model;
using HomeSpace.Infrastructure.Model;

namespace HomeSpace.Database.Repository;

public interface IFileNodeRepository
{
    Task CreateRootNode(long userId);

    Task<PagedResult<(FileNode FileNode, bool IsFavorite)>> GetNodes(
        long userId, long parentId, int page, int pageSize, string sortColumn, SortDirection sortDirection);

    Task<FileNode> GetNode(long userId, long id);
    Task<FileNode?> GetNode(long userId, long parentId, string name);
    Task<FileNode> CreateNode(long userId, long parentId, string name, NodeType nodeType, string path, string mimeType, long size);
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

        const string insertSql = @"insert into file_nodes 
            (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version)
            values (0, $1, 'ROOT', null, 0, '/', 'inode/directory', $2, 0, 1)";

        await access.ExecuteNonQuery(sequenceSql);
        await access.ExecuteNonQuery(insertSql,
            DbParameter.Create(userId),
            DbParameter.Create(DateTime.UtcNow)
        );
    }

    public async Task<PagedResult<(FileNode FileNode, bool IsFavorite)>> GetNodes(
        long userId, long parentId, int page, int pageSize, string sortColumn, SortDirection sortDirection)
    {
        var sorting = $"{sortColumn} {(sortDirection == SortDirection.Asc ? "asc" : "desc")}";
        const string totalCountSql = "select count(1) from file_nodes f where f.user_id = $1 and f.parent_id = $2";
        var totalCount = await access.ExecuteScalar<long>(totalCountSql,
            DbParameter.Create(userId),
            DbParameter.Create(parentId));
        var sql =
            $@"
select f.id, f.user_id, f.title, f.parent_id, f.node_type, f.filesystem_path, f.mime_type, f.modified_at, f.node_size, f.node_version, f.is_favorite from (
select fn.id, fn.user_id, fn.title, fn.parent_id, fn.node_type, fn.filesystem_path, fn.mime_type, fn.modified_at, fn.node_size, fn.node_version,
        case 
            when ffn.id is null then false
            else true
        end is_favorite
    from file_nodes fn
    left join favorite_nodes ffn on fn.id = ffn.id and fn.user_id = ffn.user_id  
    where fn.user_id = $1 and fn.parent_id = $2) as f
    order by node_type, {sorting}
    offset $3 limit $4";
        var pageData = new List<(FileNode, bool)>(pageSize);
        var offset = (page - 1) * pageSize;
        await foreach (var (node, isFavorite) in access.Query(sql,
                           reader => (FileNode.FromReader(reader), reader.GetFieldValue<bool>(10)),
                           DbParameter.Create(userId),
                           DbParameter.Create(parentId),
                           DbParameter.Create(offset),
                           DbParameter.Create(pageSize)))
        {
            pageData.Add((node, isFavorite));
        }

        return new PagedResult<(FileNode, bool IsFavorite)>(page, pageSize, totalCount, pageData);
    }

    public async Task<FileNode> GetNode(long userId, long id)
    {
        const string sql =
            @"select f.id, f.user_id, f.title, f.parent_id, f.node_type, f.filesystem_path, f.mime_type, f.modified_at, f.node_size, f.node_version
    from file_nodes f
    where f.user_id = $1 and f.id = $2";
        return await access.QueryOne(sql, FileNode.FromReader,
            DbParameter.Create(userId),
            DbParameter.Create(id));
    }

    public async Task<FileNode?> GetNode(long userId, long parentId, string name)
    {
        const string sql =
            @"select f.id, f.user_id, f.title, f.parent_id, f.node_type, f.filesystem_path, f.mime_type, f.modified_at, f.node_size, f.node_version
    from file_nodes f
    where f.user_id = $1 and f.parent_id = $2 and f.title = $3";
        return await access.QueryOptional(sql, FileNode.FromReader,
            DbParameter.Create(userId),
            DbParameter.Create(parentId),
            DbParameter.Create(name)
        );
    }
    
    public async Task<FileNode> CreateNode(long userId, long parentId, string name, NodeType nodeType, string path, string mimeType, long size)
    {
        var sql =
            $@"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version)
        values ({NextValSql(userId)}, $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id";
        var createTime = DateTime.UtcNow;
        var id = await access.ExecuteScalar<long>(sql,
            DbParameter.Create(userId),
            DbParameter.Create(name),
            DbParameter.Create(parentId),
            DbParameter.Create((short)nodeType),
            DbParameter.Create(path),
            DbParameter.Create(mimeType),
            DbParameter.Create(createTime),
            DbParameter.Create(size),
            DbParameter.Create(1)
        );
        return new FileNode(id, userId, name, parentId, nodeType, path, mimeType, createTime, size, 1);
    }

    private string NextValSql(long userId)
    {
        return $"nextval('file_nodes_user_{userId}')";
    }
}