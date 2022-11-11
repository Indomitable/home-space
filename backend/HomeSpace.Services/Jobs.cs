namespace HomeSpace.Services;

public interface IJob
{
    string Name { get; }
    Task Execute(CancellationToken cancellationToken);
}

public interface IRepeatableJob: IJob
{
    /// <summary>
    /// Time to wait until first execute
    /// </summary>
    TimeSpan Delay { get; }
    
    /// <summary>
    /// Repeatable interval to execute job.
    /// </summary>
    TimeSpan Interval { get; }
}
