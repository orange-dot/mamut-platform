using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Orchestration.Infrastructure.External;

/// <summary>
/// Client for making external API calls with retry and error handling.
/// </summary>
public interface IExternalApiClient
{
    Task<TResponse?> GetAsync<TResponse>(string url, Dictionary<string, string>? headers = null, CancellationToken cancellationToken = default);
    Task<TResponse?> PostAsync<TRequest, TResponse>(string url, TRequest request, Dictionary<string, string>? headers = null, CancellationToken cancellationToken = default);
    Task<TResponse?> PutAsync<TRequest, TResponse>(string url, TRequest request, Dictionary<string, string>? headers = null, CancellationToken cancellationToken = default);
    Task DeleteAsync(string url, Dictionary<string, string>? headers = null, CancellationToken cancellationToken = default);
}

/// <summary>
/// Implementation of external API client.
/// </summary>
public class ExternalApiClient : IExternalApiClient
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<ExternalApiClient> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public ExternalApiClient(HttpClient httpClient, ILogger<ExternalApiClient> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<TResponse?> GetAsync<TResponse>(
        string url,
        Dictionary<string, string>? headers = null,
        CancellationToken cancellationToken = default)
    {
        var request = CreateRequest(HttpMethod.Get, url, headers);

        _logger.LogDebug("GET {Url}", url);
        var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadFromJsonAsync<TResponse>(JsonOptions, cancellationToken);
    }

    public async Task<TResponse?> PostAsync<TRequest, TResponse>(
        string url,
        TRequest requestBody,
        Dictionary<string, string>? headers = null,
        CancellationToken cancellationToken = default)
    {
        var request = CreateRequest(HttpMethod.Post, url, headers);
        request.Content = new StringContent(
            JsonSerializer.Serialize(requestBody, JsonOptions),
            Encoding.UTF8,
            "application/json");

        _logger.LogDebug("POST {Url}", url);
        var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadFromJsonAsync<TResponse>(JsonOptions, cancellationToken);
    }

    public async Task<TResponse?> PutAsync<TRequest, TResponse>(
        string url,
        TRequest requestBody,
        Dictionary<string, string>? headers = null,
        CancellationToken cancellationToken = default)
    {
        var request = CreateRequest(HttpMethod.Put, url, headers);
        request.Content = new StringContent(
            JsonSerializer.Serialize(requestBody, JsonOptions),
            Encoding.UTF8,
            "application/json");

        _logger.LogDebug("PUT {Url}", url);
        var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadFromJsonAsync<TResponse>(JsonOptions, cancellationToken);
    }

    public async Task DeleteAsync(
        string url,
        Dictionary<string, string>? headers = null,
        CancellationToken cancellationToken = default)
    {
        var request = CreateRequest(HttpMethod.Delete, url, headers);

        _logger.LogDebug("DELETE {Url}", url);
        var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();
    }

    private static HttpRequestMessage CreateRequest(
        HttpMethod method,
        string url,
        Dictionary<string, string>? headers)
    {
        var request = new HttpRequestMessage(method, url);

        if (headers != null)
        {
            foreach (var header in headers)
            {
                request.Headers.TryAddWithoutValidation(header.Key, header.Value);
            }
        }

        return request;
    }
}
