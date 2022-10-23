namespace HomeSpace.Api;

public static partial class ServiceBuilder
{
    public static readonly string CorsDevPolicyName = "DevServer"; 
    public static IServiceCollection AddHomeSpaceCors(this IServiceCollection serviceCollection, IHostEnvironment environment)
    {
        if (environment.IsDevelopment())
        {
            serviceCollection.AddCors(options =>
            {
                options.AddPolicy(CorsDevPolicyName, policy =>
                {
                    policy.WithOrigins("http://127.0.0.1:5173")
                        .AllowAnyMethod()
                        .AllowAnyHeader()
                        .AllowCredentials();
                });
            });
        }

        return serviceCollection;
    }

    public static void UseHomeSpaceCors(this IApplicationBuilder app, IHostEnvironment environment)
    {
        if (environment.IsDevelopment())
        {
            app.UseCors(CorsDevPolicyName);
        }
    }
}