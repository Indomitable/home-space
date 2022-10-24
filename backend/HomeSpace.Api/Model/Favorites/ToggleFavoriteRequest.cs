namespace HomeSpace.Api.Model.Favorites;

public sealed record ToggleFavoriteRequest(long Id, bool Favorite);