namespace HomeSpace.Api.Model.Files;

public enum CopyNodeResultType
{
    CopyFileOverFolderError,
    CopyFolderOverFileError,
    Success
}

public record CopyNodeResult(CopyNodeResultType Type, FileNodeResponse? Node);

public record MoveNodeResult(CopyNodeResultType CopyResultType, bool Deleted, FileNodeResponse? Node);

public enum RenameNodeResultType
{
    NodeWithSameNameExist,
    Success
}

public record RenameNodeResult(RenameNodeResultType Type, FileNodeResponse? Node);

public record RenameNodeRequest(long Id, string Name);

public record CopyNodeRequest(IReadOnlyCollection<long> Nodes, long ParentId);

public record MoveNodeRequest(IReadOnlyCollection<long> Nodes, long ParentId);
