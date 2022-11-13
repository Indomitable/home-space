using HomeSpace.Api.Managers;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace HomeSpace.Api.Controllers;

[Authorize]
[ApiController]
[Route("api/trash")]
public class TrashController
{
    private readonly ITrashManager manager;

    public TrashController(ITrashManager manager)
    {
        this.manager = manager;
    }
}