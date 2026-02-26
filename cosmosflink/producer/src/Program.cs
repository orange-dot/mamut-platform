using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.DependencyInjection;
using CosmosFlinkProducer.Configuration;
using CosmosFlinkProducer.Services;

namespace CosmosFlinkProducer;

class Program
{
    static async Task Main(string[] args)
    {
        Console.WriteLine("CosmosDB Flink Producer starting...");

        try
        {
            // Load configuration from environment variables
            var config = ProducerConfiguration.FromEnvironment();
            config.Validate();

            // Create host builder
            var host = Host.CreateDefaultBuilder(args)
                .ConfigureServices((context, services) =>
                {
                    services.AddSingleton(config);
                    services.AddSingleton<CosmosProducerService>();
                    services.AddHostedService<ProducerHostedService>();
                })
                .ConfigureLogging(logging =>
                {
                    logging.ClearProviders();
                    logging.AddConsole();
                    logging.SetMinimumLevel(LogLevel.Information);
                })
                .Build();

            Console.WriteLine($"Configuration loaded:");
            Console.WriteLine($"  Cosmos URI: {config.CosmosUri}");
            Console.WriteLine($"  Database: {config.CosmosDatabase}");
            Console.WriteLine($"  Source Container: {config.CosmosSourceContainer}");
            Console.WriteLine($"  Rate: {config.RateMessagesPerSecond} messages/second");
            Console.WriteLine($"  Region: {config.Region}");
            Console.WriteLine($"  Source: {config.SourceIdentifier}");

            // Start the host
            await host.RunAsync();
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Application terminated unexpectedly: {ex.Message}");
            Console.WriteLine(ex.StackTrace);
            Environment.Exit(1);
        }
    }
}

public class ProducerHostedService : BackgroundService
{
    private readonly CosmosProducerService _producerService;
    private readonly ILogger<ProducerHostedService> _logger;
    private readonly IHostApplicationLifetime _hostApplicationLifetime;

    public ProducerHostedService(
        CosmosProducerService producerService,
        ILogger<ProducerHostedService> logger,
        IHostApplicationLifetime hostApplicationLifetime)
    {
        _producerService = producerService;
        _logger = logger;
        _hostApplicationLifetime = hostApplicationLifetime;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            _logger.LogInformation("Producer hosted service starting");
            
            // Handle graceful shutdown
            stoppingToken.Register(() =>
            {
                _logger.LogInformation("Producer hosted service is stopping due to cancellation request");
            });

            await _producerService.StartProducingAsync(stoppingToken);
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation("Producer hosted service was cancelled");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Producer hosted service failed");
            _hostApplicationLifetime.StopApplication();
        }
        finally
        {
            _producerService.Dispose();
        }
    }
}