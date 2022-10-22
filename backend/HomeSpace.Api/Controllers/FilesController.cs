using System.ComponentModel.DataAnnotations;
using System.Net.Mime;
using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Files;
using HomeSpace.Infrastructure.Model;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

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
    public async Task<IActionResult> GetFiles([FromQuery] GetFilesRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.GetFiles(request.ParentId, request.Page, request.PageSize, request.SortColumn, request.SortDirection, cancellationToken);
        return new OkObjectResult(result);
    }

    [HttpGet]
    [Route("{id}")]
    public async Task<IActionResult> GetFile([FromRoute] long id, CancellationToken cancellationToken)
    {
        var result = await manager.GetFile(id, cancellationToken);
        return new FileStreamResult(result.Content, result.ContentType)
        {
            FileDownloadName = result.Name
        };
    }

    [HttpPut]
    [Route("folder")]
    public async Task<IActionResult> CreateFolder([FromBody] CreateFolderRequest request)
    {
        var result = await manager.CreateFolder(request.ParentId, request.Name);
        return result.Type switch
        {
            CreateFolderResultType.FileWithSameNameExist => new ConflictObjectResult(result.Type),
            CreateFolderResultType.FolderWithSameNameExist => new ConflictObjectResult(result.Type),
            CreateFolderResultType.Success => new OkObjectResult(result.FileNode),
            _ => throw new ArgumentOutOfRangeException()
        };
    }
    
    [HttpPut]
    [Route("file")]
    public async Task<IActionResult> UploadFile([FromHeader(Name = "X-PARENT-ID"), Required] long parentId, IFormFile file, CancellationToken cancellationToken)
    {
        var result = await manager.UploadFile(parentId, file, cancellationToken);
        return result.Type switch
        {
            UploadFileResultType.FolderWithSameNameExist => new ConflictObjectResult(result.Type),
            UploadFileResultType.Success => new OkObjectResult(result.FileNode),
            _ => throw new ArgumentOutOfRangeException()
        };
    }
}
