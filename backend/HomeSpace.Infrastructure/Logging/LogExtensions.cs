using Microsoft.Extensions.Configuration;
using Serilog;
using Serilog.Configuration;
using Serilog.Events;

namespace HomeSpace.Infrastructure.Logging;

public static class LogExtensions
{
    public static LoggerConfiguration Configure(this LoggerConfiguration loggerConfiguration, IConfiguration configuration)
    {
        var logSection = configuration.GetSection("Log");
        var logConfiguration = logSection?.Get<LogConfiguration>() 
            ?? new LogConfiguration
            {
                ConsoleSink = new ConsoleSink
                {
                    LogLevel = LogEventLevel.Information
                }
            };
        if (logConfiguration.ConsoleSink is not null)
        {
            loggerConfiguration.WriteTo.UseSink(logConfiguration.ConsoleSink);
        }
        if (logConfiguration.FileSink is not null)
        {
            loggerConfiguration.WriteTo.UseSink(logConfiguration.FileSink);
        }

        loggerConfiguration.MinimumLevel.Is(logConfiguration.MinimumLevel);
        return loggerConfiguration;
    }

    private static void UseSink(this LoggerSinkConfiguration configuration, ISerilogSink sinkConfig)
    {
        sinkConfig.AddSink(configuration);
    }
}