namespace HomeSpace.Database.Model;

public enum AuthenticationType
{
    Password = 1,
}

public class UserAuthentication
{
    public long UserId { get; init; }
    public AuthenticationType Type { get; init; }
    public IAuthenticationType AuthenticationType { get; init; }
}