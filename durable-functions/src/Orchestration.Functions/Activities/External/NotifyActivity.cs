using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Activities.External;

/// <summary>
/// Notification channel types.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter<NotificationChannel>))]
public enum NotificationChannel
{
    Log,
    Webhook,
    Email,
    Teams
}

/// <summary>
/// Input for sending a notification.
/// </summary>
public sealed class NotifyInput
{
    public required NotificationChannel Channel { get; init; }
    public required string Subject { get; init; }
    public required string Message { get; init; }
    public string? Recipient { get; init; }
    public Dictionary<string, object?>? Data { get; init; }
    public string? WebhookUrl { get; init; }
}

/// <summary>
/// Output from sending a notification.
/// </summary>
public sealed class NotifyOutput
{
    public required NotificationChannel Channel { get; init; }
    public bool Success { get; init; }
    public string? Error { get; init; }
    public DateTimeOffset SentAt { get; init; }
}

/// <summary>
/// Activity that sends notifications via various channels.
/// </summary>
public class NotifyActivity
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<NotifyActivity> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public NotifyActivity(IHttpClientFactory httpClientFactory, ILogger<NotifyActivity> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    [Function(nameof(NotifyActivity))]
    public async Task<NotifyOutput> Run([ActivityTrigger] NotifyInput input)
    {
        _logger.LogInformation(
            "Sending notification via {Channel}: {Subject}",
            input.Channel, input.Subject);

        try
        {
            var success = input.Channel switch
            {
                NotificationChannel.Log => SendLogNotification(input),
                NotificationChannel.Webhook => await SendWebhookNotificationAsync(input),
                NotificationChannel.Email => await SendEmailNotificationAsync(input),
                NotificationChannel.Teams => await SendTeamsNotificationAsync(input),
                _ => throw new ArgumentException($"Unknown notification channel: {input.Channel}")
            };

            return new NotifyOutput
            {
                Channel = input.Channel,
                Success = success,
                SentAt = DateTimeOffset.UtcNow
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to send notification via {Channel}", input.Channel);
            return new NotifyOutput
            {
                Channel = input.Channel,
                Success = false,
                Error = ex.Message,
                SentAt = DateTimeOffset.UtcNow
            };
        }
    }

    private bool SendLogNotification(NotifyInput input)
    {
        _logger.LogWarning(
            "NOTIFICATION [{Subject}]: {Message}",
            input.Subject, input.Message);
        return true;
    }

    private async Task<bool> SendWebhookNotificationAsync(NotifyInput input)
    {
        if (string.IsNullOrEmpty(input.WebhookUrl))
        {
            throw new ArgumentException("WebhookUrl is required for webhook notifications");
        }

        var client = _httpClientFactory.CreateClient();
        var payload = new
        {
            subject = input.Subject,
            message = input.Message,
            timestamp = DateTimeOffset.UtcNow,
            data = input.Data
        };

        var json = JsonSerializer.Serialize(payload, JsonOptions);
        var content = new StringContent(json, Encoding.UTF8, "application/json");
        var response = await client.PostAsync(input.WebhookUrl, content);
        return response.IsSuccessStatusCode;
    }

    private Task<bool> SendEmailNotificationAsync(NotifyInput input)
    {
        // Placeholder for email integration
        _logger.LogInformation(
            "Email notification to {Recipient}: {Subject}",
            input.Recipient, input.Subject);
        return Task.FromResult(true);
    }

    private async Task<bool> SendTeamsNotificationAsync(NotifyInput input)
    {
        if (string.IsNullOrEmpty(input.WebhookUrl))
        {
            throw new ArgumentException("WebhookUrl is required for Teams notifications");
        }

        var client = _httpClientFactory.CreateClient();

        // Teams webhook payload format
        var teamsPayload = new
        {
            title = input.Subject,
            text = input.Message
        };

        var json = JsonSerializer.Serialize(teamsPayload, JsonOptions);
        var content = new StringContent(json, Encoding.UTF8, "application/json");
        var response = await client.PostAsync(input.WebhookUrl, content);
        return response.IsSuccessStatusCode;
    }
}
