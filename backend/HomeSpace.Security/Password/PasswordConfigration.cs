namespace HomeSpace.Security.Password;

public record PasswordConfiguration
{
    public int DegreeOfParallelism { get; init; } = 4;
    public int MemorySize { get; init; } = 4096;
    public int Iterations { get; init; } = 40;
    public string AssociatedData { get; init; } = "";
}