using HomeSpace.Api.Managers;
using HomeSpace.Api.Model.Files;
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
        var result = await manager.GetNodes(request.ParentId, request.Page, request.PageSize, request.SortColumn, request.SortDirection, cancellationToken);
        return new OkObjectResult(result);
    }

    [HttpGet]
    [Route("download")]
    public async Task<IActionResult> GetFile([FromQuery] long[] id, CancellationToken cancellationToken)
    {
        var result = await manager.GetNodesContent(id, cancellationToken);
        return new FileStreamResult(result.Content, result.ContentType)
        {
            FileDownloadName = result.Name
        };
    }

    [HttpGet]
    [Route("parents/{id}")]
    public IAsyncEnumerable<FileNodeResponse> GetParentNodes([FromRoute] long id, CancellationToken cancellationToken)
    {
        return manager.GetParents(id, cancellationToken);
    }

    [HttpPut]
    [Route("folder")]
    public async Task<IActionResult> CreateFolder([FromBody] CreateFolderRequest request)
    {
        var result = await manager.CreateFolder(request.ParentId, request.Name);
        return result.Type switch
        {
            CreateFolderResultType.FileWithSameNameExist => new ConflictObjectResult(result),
            CreateFolderResultType.FolderWithSameNameExist => new ConflictObjectResult(result),
            CreateFolderResultType.Success => new OkObjectResult(result.FileNode),
            _ => throw new ArgumentOutOfRangeException()
        };
    }
    
    [RequestFormLimits(KeyLengthLimit = int.MaxValue, ValueLengthLimit = int.MaxValue, MultipartBodyLengthLimit = int.MaxValue)]
    [RequestSizeLimit(int.MaxValue)]
    [HttpPut]
    [Route("upload")]
    public async Task<IActionResult> UploadFileChunk([FromForm] UploadFileChunkRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.UploadFileChunk(request.Id, request.File, request.Chunk, request.TotalChunks, cancellationToken);
        return new ContentResult
        {
            Content = result,
            ContentType = "plain/text",
            StatusCode = StatusCodes.Status200OK
        };
    }
    
    [RequestFormLimits(KeyLengthLimit = int.MaxValue, ValueLengthLimit = int.MaxValue, MultipartBodyLengthLimit = int.MaxValue)]
    [RequestSizeLimit(int.MaxValue)]
    [HttpPut]
    [Route("upload-last")]
    public async Task<IActionResult> UploadLastFileChunk([FromForm] UploadLastFileChunkRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.UploadLastFileChunk(request.Id, 
            request.ParentId, 
            request.File,
            request.FileName,
            request.MimeType,
            request.FileSize, request.TotalChunks, request.FileHash, cancellationToken);
        return result.Type switch
        {
            UploadFileResultType.FolderWithSameNameExist => new ConflictObjectResult(result.Type),
            UploadFileResultType.UploadError => new BadRequestObjectResult(result.Type),
            UploadFileResultType.Success => new OkObjectResult(result.FileNode),
            _ => throw new ArgumentOutOfRangeException()
        };
    }

    [HttpPost]
    [Route("rename")]
    public async Task<IActionResult> RenameFile([FromBody] RenameNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.RenameNode(request.Id, request.Name, cancellationToken);
        return result.Type switch
        {
            RenameNodeResultType.NodeWithSameNameExist => new ConflictObjectResult(result.Type),
            RenameNodeResultType.Success => new OkObjectResult(result.Node),
            _ => throw new ArgumentOutOfRangeException()
        };
    }

    [HttpPost]
    [Route("copy")]
    public async Task<IActionResult> CopyNodes([FromBody] CopyNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.CopyNodes(request.Nodes, request.ParentId, cancellationToken);
        return new OkObjectResult(result);
    }

    [HttpPost]
    [Route("move")]
    public async Task<IActionResult> MoveNodes([FromBody] MoveNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.MoveNodes(request.Nodes, request.ParentId, cancellationToken);
        return new OkObjectResult(result);
    }
}
