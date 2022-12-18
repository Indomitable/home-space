using Autofac;

namespace HomeSpace.Operations;

public class OperationsModule: Module
{
    protected override void Load(ContainerBuilder builder)
    {
        builder.RegisterType<TransactionFactory>().As<ITransactionFactory>().SingleInstance();
        builder.RegisterType<OperationFactory>().As<IOperationFactory>().SingleInstance();
    }
}
