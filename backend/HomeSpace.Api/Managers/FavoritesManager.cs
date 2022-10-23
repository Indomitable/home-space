using HomeSpace.Database.Repository;
using HomeSpace.Security.Services;

namespace HomeSpace.Api.Managers;

public interface IFavoritesManager
{
    Task ToggleFavorite(long nodeId, bool favorite, CancellationToken cancellationToken);
}

internal sealed class FavoritesManager : IFavoritesManager
{
    private readonly ICurrentUserProvider userProvider;
    private readonly IFavoritesRepository repository;

    public FavoritesManager(ICurrentUserProvider userProvider, IFavoritesRepository repository)
    {
        this.userProvider = userProvider;
        this.repository = repository;
    }

    public Task ToggleFavorite(long nodeId, bool favorite, CancellationToken cancellationToken)
    {
        var user = userProvider.RequireAuthorizedUser();
        return favorite
            ? repository.SetFavorite(user.Id, nodeId, cancellationToken)
            : repository.UnsetFavorite(user.Id, nodeId, cancellationToken);
    }
    
}