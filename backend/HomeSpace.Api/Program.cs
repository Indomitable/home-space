using HomeSpace.Api;
using HomeSpace.Database;
using HomeSpace.Files;
using HomeSpace.Infrastructure.Configuration;
using HomeSpace.Infrastructure.Logging;
using HomeSpace.Security;
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
    builder.Services.AddControllers();
    builder.Services
        .AddEndpointsApiExplorer()
        .AddSwaggerGen()
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
    }
    app.UseHttpsRedirection();
    app.UseRouting();
    app.UseAuthentication();
    app.UseAuthorization();
    app.UseEndpoints(routeBuilder =>
    {
        routeBuilder.MapControllers();
    });
    app.MapFallbackToFile("index.html");;
    
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
