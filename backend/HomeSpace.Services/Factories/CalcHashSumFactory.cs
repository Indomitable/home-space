using HomeSpace.Database.Repository;
using HomeSpace.Files.Services;
using HomeSpace.Services.Files;

namespace HomeSpace.Services.Factories;

public interface ICalcHashSumFactory
{
    IJob CreateCalcFileNodeHashSumJob(long userId, long fileNodeId);
}

internal sealed class CalcHashSumFactory : ICalcHashSumFactory
{
    private readonly IFilesService filesService;
    private readonly IFileNodeRepository repository;

    public CalcHashSumFactory(IFilesService filesService, IFileNodeRepository repository)
    {
        this.filesService = filesService;
        this.repository = repository;
    }

    public IJob CreateCalcFileNodeHashSumJob(long userId, long fileNodeId)
    {
        return new CalcFileNodeHashSum(userId, fileNodeId, filesService, repository);
    }
}