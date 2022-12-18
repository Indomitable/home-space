using Autofac;
using Autofac.Extensions.DependencyInjection;
using FluentValidation.AspNetCore;
using HomeSpace.Api;
using HomeSpace.Api.Formatters;
using HomeSpace.Database;
using HomeSpace.Files;
using HomeSpace.Infrastructure.Configuration;
using HomeSpace.Infrastructure.Json;
using HomeSpace.Infrastructure.Logging;
using HomeSpace.Operations;
using HomeSpace.Security;
using HomeSpace.Services;
using Serilog;

var configuration = ConfigurationFactory.Create();
Log.Logger = new LoggerConfiguration()
    .Configure(configuration)
    .CreateBootstrapLogger();
Log.Information("Start Home Space");
try
{
    var builder = WebApplication.CreateBuilder(args);
    builder.Host
        .UseServiceProviderFactory(new AutofacServiceProviderFactory())
        .UseSerilog(Log.Logger)
        .UseSystemd();
    builder.Configuration.AddConfig(configuration);
    builder.Host.ConfigureContainer<ContainerBuilder>((_, container) =>
    {
        container.RegisterModule<ServicesModule>();
        container.RegisterModule<SecurityModule>();
        container.RegisterModule<FilesModule>();
        container.RegisterModule<DatabaseModule>();
        container.RegisterModule<OperationsModule>();
        container.RegisterModule<ApiModule>();
    });
    builder.Services
        .AddControllers(o =>
        {
            o.InputFormatters.Add(new TextInputFormatter());
        })
        .AddJsonOptions(options =>
        {
            JsonSerializer.Configure(options.JsonSerializerOptions);
        });
    
    builder.Services
        .AddEndpointsApiExplorer()
        .AddHttpContextAccessor()
        .AddSwagger()
        .AddFluentValidationAutoValidation()
        .AddJwtAuthentication(configuration);

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
