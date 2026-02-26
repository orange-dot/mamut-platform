using CosmosFlinkConsumer.Configuration;
using CosmosFlinkConsumer.Services;
using Microsoft.Extensions.Logging;

namespace CosmosFlinkConsumer;

class Program
{
    static async Task Main(string[] args)
    {
        Console.WriteLine("CosmosDB Flink Consumer starting...");

        try
        {
            // Load configuration from environment variables
            var config = ConsumerConfiguration.FromEnvironment();
            config.Validate();

            var builder = WebApplication.CreateBuilder(args);

            // Configure services
            builder.Services.AddSingleton(config);
            builder.Services.AddSingleton<EventProcessingService>();
            builder.Services.AddControllers();
            builder.Services.AddEndpointsApiExplorer();
            builder.Services.AddSwaggerGen();

            // Configure logging
            builder.Logging.ClearProviders();
            builder.Logging.AddConsole();
            builder.Logging.SetMinimumLevel(LogLevel.Information);

            // Configure Kestrel
            builder.WebHost.ConfigureKestrel(serverOptions =>
            {
                serverOptions.ListenAnyIP(config.HttpPort);
            });

            var app = builder.Build();

            Console.WriteLine($"Configuration loaded:");
            Console.WriteLine($"  HTTP Host: {config.HttpHost}");
            Console.WriteLine($"  HTTP Port: {config.HttpPort}");
            Console.WriteLine($"  TCP Port: {config.TcpPort}");
            Console.WriteLine($"  Serialization Format: {config.SerializationFormat}");
            Console.WriteLine($"  Consumer ID: {config.ConsumerIdentifier}");
            Console.WriteLine($"  Max Batch Size: {config.MaxBatchSize}");

            // Configure the HTTP request pipeline
            if (app.Environment.IsDevelopment())
            {
                app.UseSwagger();
                app.UseSwaggerUI();
            }

            app.UseRouting();
            app.MapControllers();

            // Add a simple root endpoint
            app.MapGet("/", () => new
            {
                service = "CosmosDB Flink Consumer",
                status = "running",
                timestamp = DateTime.UtcNow,
                endpoints = new
                {
                    events = "/api/events",
                    batch = "/api/events/batch",
                    health = "/api/events/health",
                    statistics = "/api/events/statistics"
                }
            });

            app.MapGet("/health", (EventProcessingService eventProcessingService) =>
            {
                var stats = eventProcessingService.GetStatistics();
                return Results.Ok(new
                {
                    status = "healthy",
                    timestamp = DateTime.UtcNow,
                    statistics = stats
                });
            });

            Console.WriteLine($"Consumer starting on http://{config.HttpHost}:{config.HttpPort}");
            Console.WriteLine("Available endpoints:");
            Console.WriteLine($"  GET  / - Service information");
            Console.WriteLine($"  GET  /health - Health check");
            Console.WriteLine($"  POST /api/events - Receive single event");
            Console.WriteLine($"  POST /api/events/batch - Receive event batch");
            Console.WriteLine($"  GET  /api/events/health - Detailed health check");
            Console.WriteLine($"  GET  /api/events/statistics - Processing statistics");

            await app.RunAsync();
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Application terminated unexpectedly: {ex.Message}");
            Console.WriteLine(ex.StackTrace);
            Environment.Exit(1);
        }
    }
}