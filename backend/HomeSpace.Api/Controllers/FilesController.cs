using System.Runtime.CompilerServices;
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
    [Route("{id}")]
    public async Task<IActionResult> GetNodeById([FromRoute] long id, CancellationToken cancellationToken)
    {
        var result = await manager.GetNodeById(id, cancellationToken);
        if (result is null)
        {
            return new NotFoundResult();
        }
        return new OkObjectResult(FileNodeResponse.Map(result));
    }
    
    [HttpGet]
    [Route("node")]
    public async Task<IActionResult> GetNodeByPath([FromQuery] string path, CancellationToken cancellationToken)
    {
        var result = await manager.GetNodeByPath(path, cancellationToken);
        if (result is null)
        {
            return new NotFoundResult();
        }
        return new OkObjectResult(FileNodeResponse.Map(result));
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
    public async IAsyncEnumerable<FileNodeResponse> GetParentNodes([FromRoute] long id, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var node in manager.GetParents(id, cancellationToken))
        {
            yield return FileNodeResponse.Map(node);
        }
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
            CreateFolderResultType.Success => new OkObjectResult(FileNodeResponse.Map(result.FileNode)),
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
            UploadFileResultType.ParentNotFound => new NotFoundResult(),
            UploadFileResultType.FolderWithSameNameExist => new ConflictObjectResult(result.Type),
            UploadFileResultType.UploadError => new BadRequestObjectResult(result.Type),
            UploadFileResultType.Success => new OkObjectResult(FileNodeResponse.Map(result.FileNode)),
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
            RenameNodeResultType.NodeNotFound => new NotFoundResult(),
            RenameNodeResultType.NodeWithSameNameExist => new ConflictObjectResult(result.Type),
            RenameNodeResultType.Success => new OkObjectResult(FileNodeResponse.Map(result.Node)),
            _ => throw new ArgumentOutOfRangeException()
        };
    }

    [HttpPost]
    [Route("copy")]
    public async Task<IActionResult> CopyNodes([FromBody] CopyNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.CopyNodes(request.Nodes, request.ParentId, cancellationToken);
        if (result.All(r => r.Value.Type is CopyNodeResultType.SourceNotFound or CopyNodeResultType.DestinationNotFound))
        {
            return new NotFoundResult();
        }
        return new OkObjectResult(result.ToDictionary(_ => _.Key, _ => new
        {
            _.Value.Type,
            Node = FileNodeResponse.Map(_.Value.Node)
        }));
    }

    [HttpPost]
    [Route("move")]
    public async Task<IActionResult> MoveNodes([FromBody] MoveNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.MoveNodes(request.Nodes, request.ParentId, cancellationToken);
        if (result.All(r => r.Value.CopyResultType is CopyNodeResultType.SourceNotFound or CopyNodeResultType.DestinationNotFound))
        {
            return new NotFoundResult();
        }
        return new OkObjectResult(result.ToDictionary(_ => _.Key, _ => new
        {
            Type = _.Value.CopyResultType,
            Node = FileNodeResponse.Map(_.Value.Node)
        }));
    }

    [HttpDelete]
    [Route("")]
    public async Task<IActionResult> DeleteNode([FromBody] DeleteNodeRequest request, CancellationToken cancellationToken)
    {
        var result = await manager.MoveNodesToTrash(request.Nodes, cancellationToken);
        if (result.All(r => r.Value == DeleteNodeResult.NodeNotExist))
        {
            return new NotFoundResult();
        }
        return new OkObjectResult(result);
    }
}
