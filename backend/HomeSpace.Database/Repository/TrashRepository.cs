using HomeSpace.Database.Model;

namespace HomeSpace.Database.Repository;

public interface ITrashRepository
{
    Task MoveNodeFromVersionToTrash(FileNode node, string fileName, int trashVersion, CancellationToken cancellationToken);
    
    Task MoveNodeToTrash(FileNode node, string fileName, int trashVersion, CancellationToken cancellationToken);

    /// <summary>
    /// Return all trash items for a node located by its path
    /// </summary>
    IAsyncEnumerable<TrashNode> GetFileTrashNodes(long userId, string path, CancellationToken cancellationToken);

}

public class TrashRepository : ITrashRepository
{
    private readonly IDbAccess dbAccess;

    public TrashRepository(IDbAccess dbAccess)
    {
        this.dbAccess = dbAccess;
    }

    public async Task MoveNodeFromVersionToTrash(FileNode node, string fileName, int trashVersion,
        CancellationToken cancellationToken)
    {
        await using var tran = await dbAccess.StartTransaction();
        await CreateTrashNodeFromFileNode(tran, node, fileName, trashVersion, cancellationToken);

        const string deleteSql = @"delete from file_versions where user_id = $1 and id = $2 and node_version = $3";
        await tran.ExecuteNonQuery(deleteSql, cancellationToken,
            DbParameter.Create(node.UserId),
            DbParameter.Create(node.Id),
            DbParameter.Create(node.Version)
        );
        
        await tran.Commit(cancellationToken);

    }

    public async Task MoveNodeToTrash(FileNode node, string fileName, int trashVersion, CancellationToken cancellationToken)
    {
        await using var tran = await dbAccess.StartTransaction();
        await CreateTrashNodeFromFileNode(tran, node, fileName, trashVersion, cancellationToken);

        const string deleteSql = @"delete from file_nodes where user_id = $1 and id = $2";
        await tran.ExecuteNonQuery(deleteSql, cancellationToken,
            DbParameter.Create(node.UserId),
            DbParameter.Create(node.Id)
        );
        
        await tran.Commit(cancellationToken);
    }

    private async Task CreateTrashNodeFromFileNode(IDbTransaction transaction, FileNode node, string fileName, int trashVersion,
        CancellationToken cancellationToken)
    {
        const string insertSql = @"insert into trash_box 
        (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, version_created_at, deleted_at, node_size, node_version, file_name)
        values ($2, $1, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
        await transaction.ExecuteNonQuery(insertSql, cancellationToken,
            DbParameter.Create(node.UserId),
            DbParameter.Create(node.Id),
            DbParameter.Create(node.Title),
            DbParameter.Create(node.ParentId.GetValueOrDefault(0)),
            DbParameter.Create((short)node.NodeType),
            DbParameter.Create(node.FileSystemPath),
            DbParameter.Create(node.MimeType),
            DbParameter.Create(node.ModifiedAt),
            DbParameter.Create(DateTime.UtcNow),
            DbParameter.Create(node.Size),
            DbParameter.Create(node.Version + trashVersion), // When insert add version if has same node in trash.
            DbParameter.Create(fileName)
        );
    }

    public IAsyncEnumerable<TrashNode> GetFileTrashNodes(long userId, string path, CancellationToken cancellationToken)
    {
        const string sql =
            @"select id, user_id, title, parent_id, node_type, filesystem_path, mime_type, version_created_at, deleted_at, node_size, node_version, file_name
    from trash_box 
    where user_id = $1 and filesystem_path = $2";
        return dbAccess.Query(sql, TrashNode.FromReader, cancellationToken,
            DbParameter.Create(userId),
            DbParameter.Create(path)
        );
    }
}