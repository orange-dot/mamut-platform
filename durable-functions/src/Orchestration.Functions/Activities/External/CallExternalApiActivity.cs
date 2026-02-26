using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Activities.External;

/// <summary>
/// Input for calling an external API.
/// </summary>
public sealed class CallExternalApiInput
{
    public required string Url { get; init; }
    public string Method { get; init; } = "GET";
    public Dictionary<string, string>? Headers { get; init; }
    public object? Body { get; init; }
    public int TimeoutSeconds { get; init; } = 30;
    public string? IdempotencyKey { get; init; }
}

/// <summary>
/// Output from calling an external API.
/// </summary>
public sealed class CallExternalApiOutput
{
    public required string Url { get; init; }
    public int StatusCode { get; init; }
    public bool IsSuccess { get; init; }
    public string? ResponseBody { get; init; }
    public Dictionary<string, string>? ResponseHeaders { get; init; }
    public string? Error { get; init; }
}

/// <summary>
/// Activity that calls an external HTTP API.
/// </summary>
public class CallExternalApiActivity
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<CallExternalApiActivity> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public CallExternalApiActivity(IHttpClientFactory httpClientFactory, ILogger<CallExternalApiActivity> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    [Function(nameof(CallExternalApiActivity))]
    public async Task<CallExternalApiOutput> Run([ActivityTrigger] CallExternalApiInput input)
    {
        _logger.LogInformation(
            "Calling external API: {Method} {Url}",
            input.Method, input.Url);

        var client = _httpClientFactory.CreateClient();
        client.Timeout = TimeSpan.FromSeconds(input.TimeoutSeconds);

        try
        {
            var request = new HttpRequestMessage(
                new HttpMethod(input.Method),
                input.Url);

            // Add headers
            if (input.Headers != null)
            {
                foreach (var header in input.Headers)
                {
                    request.Headers.TryAddWithoutValidation(header.Key, header.Value);
                }
            }

            // Add idempotency key header if provided
            if (!string.IsNullOrEmpty(input.IdempotencyKey))
            {
                request.Headers.TryAddWithoutValidation("Idempotency-Key", input.IdempotencyKey);
            }

            // Add body for POST/PUT/PATCH
            if (input.Body != null &&
                (input.Method.Equals("POST", StringComparison.OrdinalIgnoreCase) ||
                 input.Method.Equals("PUT", StringComparison.OrdinalIgnoreCase) ||
                 input.Method.Equals("PATCH", StringComparison.OrdinalIgnoreCase)))
            {
                var json = JsonSerializer.Serialize(input.Body, JsonOptions);
                request.Content = new StringContent(json, Encoding.UTF8, "application/json");
            }

            var response = await client.SendAsync(request);
            var responseBody = await response.Content.ReadAsStringAsync();

            var responseHeaders = response.Headers
                .Concat(response.Content.Headers)
                .ToDictionary(h => h.Key, h => string.Join(", ", h.Value));

            _logger.LogInformation(
                "External API response: {StatusCode} from {Url}",
                (int)response.StatusCode, input.Url);

            return new CallExternalApiOutput
            {
                Url = input.Url,
                StatusCode = (int)response.StatusCode,
                IsSuccess = response.IsSuccessStatusCode,
                ResponseBody = responseBody,
                ResponseHeaders = responseHeaders
            };
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "HTTP request failed for {Url}", input.Url);
            return new CallExternalApiOutput
            {
                Url = input.Url,
                StatusCode = 0,
                IsSuccess = false,
                Error = ex.Message
            };
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogError(ex, "Request timed out for {Url}", input.Url);
            return new CallExternalApiOutput
            {
                Url = input.Url,
                StatusCode = 0,
                IsSuccess = false,
                Error = "Request timed out"
            };
        }
    }
}
