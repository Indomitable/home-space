using System.Net.Mime;
using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Files;
using HomeSpace.Infrastructure.Model;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Formatters;

namespace HomeSpace.Api.Controllers;

[Authorize]
[ApiController]
[Route("api/files")]
public class FilesController
{
    private readonly IFilesManager manager;

    public FilesController(IFilesManager manager)
    {
        this.manager = manager;
    }

    [HttpGet]
    [Route("")]
    public async Task<IActionResult> GetFiles(long parentId, int page, int pageSize, FileNodeSort sortColumn, SortDirection sortDirection = SortDirection.Asc)
    {
        var result = await manager.GetFiles(parentId, page, pageSize, sortColumn, sortDirection);
        return new OkObjectResult(result);
    }

    [HttpGet]
    [Route("{id}")]
    public async Task<IActionResult> GetFile([FromRoute] long id)
    {
        var result = await manager.GetFile(id);
        return new FileStreamResult(result.Content, MediaTypeNames.Application.Octet)
        {
            FileDownloadName = result.Name
        };
    }

    [HttpPut]
    [Route("folder")]
    public async Task<IActionResult> CreateFolder([FromBody] CreateFolderRequest request)
    {
        var result = await manager.CreateFolder(request.ParentId, request.Name);
        switch (result.Type)
        {
            case CreateFolderResultType.FileWithSameNameExist:
            case CreateFolderResultType.FolderWithSameNameExist:
                return new ConflictObjectResult(result.Type);
            case CreateFolderResultType.Success:
                return new OkObjectResult(result.FileNode);
            default:
                throw new ArgumentOutOfRangeException();
        }
    }
}
