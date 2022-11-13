namespace HomeSpace.Api.Model.Files;

public enum DeleteNodeResult
{
    NodeNotExist,
    Success
}

public record DeleteNodeRequest(IReadOnlyCollection<long> Nodes);
