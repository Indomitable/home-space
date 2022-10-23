using System.Net.Mime;
using HomeSpace.Api;
using HomeSpace.Database;
using HomeSpace.Files;
using HomeSpace.Infrastructure.Configuration;
using HomeSpace.Infrastructure.Json;
using HomeSpace.Infrastructure.Logging;
using HomeSpace.Security;
using Microsoft.AspNetCore.Mvc.ApplicationParts;
using Microsoft.AspNetCore.Mvc.Formatters;
using Serilog;

var configuration = ConfigurationFactory.Create();
Log.Logger = new LoggerConfiguration()
    .Configure(configuration)
    .CreateBootstrapLogger();
Log.Information("Start Home Space");
try
{
    var builder = WebApplication.CreateBuilder(args);
    builder.Host.UseSerilog(Log.Logger);
    builder.Configuration.AddConfig(configuration);
    builder.Services
        .AddControllers()
        .AddJsonOptions(options =>
        {
            JsonSerializer.Configure(options.JsonSerializerOptions);
        });
    builder.Services
        .AddEndpointsApiExplorer()
        .AddHttpContextAccessor()
        .AddSwagger()
        .AddServices()
        .AddHomeSpaceDb()
        .AddHomeSpaceFiles()
        .AddHomeSpaceSecurity(configuration);

    var app = builder.Build();
    app.EnableSwagger(configuration);

    if (app.Environment.IsProduction())
    {
        app.UseHsts();
        app.UseDefaultFiles();
        app.UseStaticFiles();
        app.MapFallbackToFile("index.html");
    }
    app.UseHttpsRedirection();
    app.UseRouting();
    app.UseAuthentication();
    app.UseAuthorization();
    app.HandleExceptions();
    
    app.UseEndpoints(routeBuilder =>
    {
        routeBuilder.MapControllers();
    });
    
    await app.RunAsync();
}
catch (Exception e)
{
    Log.Error(e, "Home space failed");
}
finally
{
    Log.Information("Stop home space");
    Log.CloseAndFlush();
}