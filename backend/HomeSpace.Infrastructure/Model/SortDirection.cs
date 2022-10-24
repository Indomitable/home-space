namespace HomeSpace.Infrastructure.Model;

public enum SortDirection
{
    Asc,
    Desc
}

public static class SortDirectionExtensions
{
    public static string GetOrderByDirection(this SortDirection sortDirection)
    {
        return sortDirection switch
        {
            SortDirection.Asc => "asc",
            SortDirection.Desc => "desc",
            _ => throw new ArgumentOutOfRangeException(nameof(sortDirection), sortDirection, null)
        };
    }
}