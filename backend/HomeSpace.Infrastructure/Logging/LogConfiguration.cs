using Serilog;
using Serilog.Configuration;
using Serilog.Events;

namespace HomeSpace.Infrastructure.Logging;

public class LogConfiguration
{
    public LogEventLevel MinimumLevel { get; set; }
    public ConsoleSink? ConsoleSink { get; init; }

    public FileSink? FileSink { get; init; }
}

public interface ISerilogSink
{
    LogEventLevel LogLevel { get; }

    void AddSink(LoggerSinkConfiguration configuration);
}

public sealed record ConsoleSink: ISerilogSink
{
    public LogEventLevel LogLevel { get; init; }

    public void AddSink(LoggerSinkConfiguration configuration)
    {
        configuration.Console(LogLevel);
    }
}

public sealed record FileSink: ISerilogSink
{
    public string FileName { get; init; }
    public LogEventLevel LogLevel { get; init; }
    
    public void AddSink(LoggerSinkConfiguration configuration)
    {
        configuration.Async(c =>
        {
            c.File(FileName, LogLevel);
        });
    }
}
