using System.Security.Cryptography;
using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;

namespace HomeSpace.Services.Files;

internal sealed class CalcFileNodeHashSum: IJob
{
    private readonly long userId;
    private readonly long fileNodeId;
    private readonly IFilesService filesService;
    private readonly IFileNodeRepository repository;

    public CalcFileNodeHashSum(long userId, long fileNodeId, IFilesService filesService, IFileNodeRepository repository)
    {
        this.userId = userId;
        this.fileNodeId = fileNodeId;
        this.filesService = filesService;
        this.repository = repository;
    }

    public string Name => "FileNodeHashSum";
    public async Task Execute(CancellationToken cancellationToken)
    {
        var fileNode = await repository.GetNode(userId, fileNodeId, cancellationToken);
        if (fileNode is not null)
        {
            await using var stream = filesService.ReadFile(userId, fileNode.FileSystemPath);
            using var sha256 = SHA256.Create();
            var hashSum = await sha256.ComputeHashAsync(stream, cancellationToken);
            await repository.UpdateNodeHashSum(userId, fileNodeId, hashSum, cancellationToken);
        }
    }
}