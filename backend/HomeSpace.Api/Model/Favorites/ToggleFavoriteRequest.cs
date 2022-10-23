namespace HomeSpace.Api.Model.Favorites;

public sealed record ToggleFavoriteRequest(long NodeId, bool Favorite);