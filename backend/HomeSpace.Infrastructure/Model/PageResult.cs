namespace HomeSpace.Infrastructure.Model;

public record PagedResult<T>(int Page, int PageSize, long TotalCount, IEnumerable<T> PageData)
{
    public PagedResult<TTo> Map<TTo>(Func<T, TTo> map)
    {
        return new PagedResult<TTo>(Page, PageSize, TotalCount, PageData.Select(map));
    }
}
