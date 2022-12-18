namespace HomeSpace.Database.Model;

public interface IAuthenticationType
{
    Task<long> Add(IDbTransaction transaction);
}
