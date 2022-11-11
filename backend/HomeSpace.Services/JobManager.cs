using Microsoft.Extensions.Hosting;

namespace HomeSpace.Services;

public interface IJobManager
{
    void QueueJob(IJob job, CancellationToken cancellationToken);
} 

class JobManager: IJobManager, IHostedService
{
    private static readonly object queueJobLock = new object();
    private readonly IEnumerable<IRepeatableJob> repeatableJobs;
    private readonly Dictionary<string, Task> runningTasks = new ();
    private readonly Dictionary<string, Queue<(IJob, CancellationToken)>> pendingJobs = new ();
    private readonly List<Task> repeatableTasks = new();
    private readonly CancellationTokenSource serviceCancellationTokenSource = new();

    public JobManager(IEnumerable<IRepeatableJob> repeatableJobs)
    {
        this.repeatableJobs = repeatableJobs;
    }
    
    public Task StartAsync(CancellationToken cancellationToken)
    {
        foreach (var repeatableJob in repeatableJobs)
        {
            var source = CancellationTokenSource.CreateLinkedTokenSource(serviceCancellationTokenSource.Token, cancellationToken);
            var task = RunRepeatableJob(repeatableJob, source.Token);
            repeatableTasks.Add(task);
        }
        return Task.CompletedTask;
    }

    public Task StopAsync(CancellationToken cancellationToken)
    {
        // Cancel main service token. 
        serviceCancellationTokenSource.Cancel();
        // Wait all tasks to be cancelled
        return Task.WhenAll(repeatableTasks.Union(runningTasks.Values));
    }

    /// <summary>
    /// Execute job, only one job of certain type can be executed.
    /// </summary>
    /// <param name="job"></param>
    /// <param name="cancellationToken"></param>
    public void QueueJob(IJob job, CancellationToken cancellationToken)
    {
        var source = CancellationTokenSource.CreateLinkedTokenSource(serviceCancellationTokenSource.Token, cancellationToken);
        InternalQueueJob(job, source.Token);
    }

    private void InternalQueueJob(IJob job, CancellationToken cancellationToken)
    {
        lock (queueJobLock)
        {
            if (runningTasks.TryGetValue(job.Name, out var runningJob))
            {
                if (runningJob.IsCompleted)
                {
                    runningTasks.Remove(job.Name);
                    runningTasks.Add(job.Name, RunJob(job, cancellationToken));
                }
                else
                {
                    AddPendingJob(job, cancellationToken);
                }
            }
            else
            {
                runningTasks.Add(job.Name, RunJob(job, cancellationToken));
            }
        }
    }

    private async Task RunRepeatableJob(IRepeatableJob repeatableJob, CancellationToken cancellationToken)
    {
        await Task.Delay(repeatableJob.Delay, cancellationToken);
        if (cancellationToken.IsCancellationRequested) return;
        await repeatableJob.Execute(cancellationToken);
        await Task.Delay(repeatableJob.Interval, cancellationToken);
        if (cancellationToken.IsCancellationRequested) return;
        await RunRepeatableJob(repeatableJob, cancellationToken);
    }

    private void AddPendingJob(IJob job, CancellationToken cancellationToken)
    {
        if (pendingJobs.TryGetValue(job.Name, out var queue))
        {
            queue.Enqueue((job, cancellationToken));
        }
        else
        {
            pendingJobs.Add(job.Name, new Queue<(IJob, CancellationToken)>(Enumerable.Repeat((job, cancellationToken), 1)));
        }
    }

    private async Task RunJob(IJob job, CancellationToken cancellationToken)
    {
        await job.Execute(cancellationToken);
        runningTasks.Remove(job.Name);
        if (cancellationToken.IsCancellationRequested) return;
        if (pendingJobs.TryGetValue(job.Name, out var queue))
        {
            if (queue.TryDequeue(out var next))
            {
                if (!next.Item2.IsCancellationRequested)
                {
                    InternalQueueJob(next.Item1, next.Item2);
                }
            }
        }
    }
}