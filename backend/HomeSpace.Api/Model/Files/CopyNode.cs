using HomeSpace.Database.Model;

namespace HomeSpace.Api.Model.Files;

public enum CopyNodeResultType
{
    SourceNotFound,
    DestinationNotFound,
    CopyFileOverFolderError,
    CopyFolderOverFileError,
    Success
}

public record CopyNodeResult(CopyNodeResultType Type, FileNode? Node = null);

public record MoveNodeResult(CopyNodeResultType CopyResultType, FileNode? Node = null);

public enum RenameNodeResultType
{
    NodeNotFound,
    NodeWithSameNameExist,
    Success,
}

public record RenameNodeResult(RenameNodeResultType Type, FileNode? Node = null);

public record RenameNodeRequest(long Id, string Name);

public record CopyNodeRequest(IReadOnlyCollection<long> Nodes, long ParentId);

public record MoveNodeRequest(IReadOnlyCollection<long> Nodes, long ParentId);
