namespace HomeSpace.Api.Model.Files;

public record CreateFolderRequest(long ParentId, string Name);