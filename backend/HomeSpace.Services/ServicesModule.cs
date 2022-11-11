using Autofac;
using Microsoft.Extensions.Hosting;

namespace HomeSpace.Services;

public class ServicesModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.RegisterType<JobManager>()
            .As<IHostedService>()
            .As<IJobManager>()
            .SingleInstance();
    }
}