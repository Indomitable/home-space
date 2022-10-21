using HomeSpace.Database;
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
        .AddHomeSpaceSecurity(configuration)
        .AddHomeSpaceDb();

    var app = builder.Build();

    // Configure the HTTP request pipeline.
    //if (app.Environment.IsDevelopment())
    {
        app.UseSwagger();
        app.UseSwaggerUI();
    }

    if (app.Environment.IsProduction())
    {
        app.UseHsts();
        app.UseStaticFiles();
    }

    app.UseHttpsRedirection();
    app.UseAuthentication();
    app.UseAuthorization();
    app.MapControllers();
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
