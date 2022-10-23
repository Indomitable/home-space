using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Favorites;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[Authorize]
[ApiController]
[Route("favorites")]
public class FavoritesController
{
    private readonly IFavoritesManager manager;

    public FavoritesController(IFavoritesManager manager)
    {
        this.manager = manager;
    }
    
    [HttpPost]
    [Route("toggle")]
    public async Task<IActionResult> Toggle(ToggleFavoriteRequest request, CancellationToken cancellationToken)
    {
        await manager.ToggleFavorite(request.NodeId, request.Favorite, cancellationToken);
        return new OkResult();
    }
}