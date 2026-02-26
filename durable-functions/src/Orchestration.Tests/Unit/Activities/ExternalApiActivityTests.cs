using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using Moq.Protected;
using System.Net;
using Orchestration.Functions.Activities.External;

namespace Orchestration.Tests.Unit.Activities;

public class CallExternalApiActivityTests
{
    private readonly Mock<IHttpClientFactory> _httpClientFactoryMock;
    private readonly Mock<ILogger<CallExternalApiActivity>> _loggerMock;
    private readonly Mock<HttpMessageHandler> _httpMessageHandlerMock;
    private readonly CallExternalApiActivity _activity;

    public CallExternalApiActivityTests()
    {
        _httpClientFactoryMock = new Mock<IHttpClientFactory>();
        _loggerMock = new Mock<ILogger<CallExternalApiActivity>>();
        _httpMessageHandlerMock = new Mock<HttpMessageHandler>();

        var httpClient = new HttpClient(_httpMessageHandlerMock.Object);
        _httpClientFactoryMock.Setup(x => x.CreateClient(It.IsAny<string>())).Returns(httpClient);

        _activity = new CallExternalApiActivity(_httpClientFactoryMock.Object, _loggerMock.Object);
    }

    [Fact]
    public async Task Run_WithSuccessfulGetRequest_ReturnsSuccessResponse()
    {
        // Arrange
        var input = new CallExternalApiInput
        {
            Url = "https://api.example.com/data",
            Method = "GET"
        };

        _httpMessageHandlerMock
            .Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .ReturnsAsync(new HttpResponseMessage
            {
                StatusCode = HttpStatusCode.OK,
                Content = new StringContent("{\"status\": \"ok\"}")
            });

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.IsSuccess.Should().BeTrue();
        result.StatusCode.Should().Be(200);
        result.Url.Should().Be("https://api.example.com/data");
    }

    [Fact]
    public async Task Run_WithServerError_ReturnsErrorResponse()
    {
        // Arrange
        var input = new CallExternalApiInput
        {
            Url = "https://api.example.com/error",
            Method = "GET"
        };

        _httpMessageHandlerMock
            .Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .ReturnsAsync(new HttpResponseMessage
            {
                StatusCode = HttpStatusCode.InternalServerError,
                Content = new StringContent("Internal Server Error")
            });

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.IsSuccess.Should().BeFalse();
        result.StatusCode.Should().Be(500);
    }

    [Fact]
    public async Task Run_WithPostRequest_SendsBodyCorrectly()
    {
        // Arrange
        var input = new CallExternalApiInput
        {
            Url = "https://api.example.com/items",
            Method = "POST",
            Body = new { name = "test", value = 123 }
        };

        HttpRequestMessage? capturedRequest = null;
        _httpMessageHandlerMock
            .Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .Callback<HttpRequestMessage, CancellationToken>((req, _) => capturedRequest = req)
            .ReturnsAsync(new HttpResponseMessage
            {
                StatusCode = HttpStatusCode.Created,
                Content = new StringContent("{\"id\": \"new-id\"}")
            });

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.IsSuccess.Should().BeTrue();
        result.StatusCode.Should().Be(201);
        capturedRequest.Should().NotBeNull();
        capturedRequest!.Method.Should().Be(HttpMethod.Post);
    }

    [Fact]
    public async Task Run_WithHeaders_SendsHeadersCorrectly()
    {
        // Arrange
        var input = new CallExternalApiInput
        {
            Url = "https://api.example.com/auth",
            Method = "GET",
            Headers = new Dictionary<string, string>
            {
                ["Authorization"] = "Bearer token123",
                ["X-Custom-Header"] = "custom-value"
            }
        };

        HttpRequestMessage? capturedRequest = null;
        _httpMessageHandlerMock
            .Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .Callback<HttpRequestMessage, CancellationToken>((req, _) => capturedRequest = req)
            .ReturnsAsync(new HttpResponseMessage
            {
                StatusCode = HttpStatusCode.OK,
                Content = new StringContent("{}")
            });

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.IsSuccess.Should().BeTrue();
        capturedRequest.Should().NotBeNull();
        capturedRequest!.Headers.Should().Contain(h => h.Key == "Authorization");
    }

    [Fact]
    public async Task Run_WithHttpRequestException_ReturnsError()
    {
        // Arrange
        var input = new CallExternalApiInput
        {
            Url = "https://api.example.com/network-error",
            Method = "GET"
        };

        _httpMessageHandlerMock
            .Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .ThrowsAsync(new HttpRequestException("Network error"));

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.IsSuccess.Should().BeFalse();
        result.StatusCode.Should().Be(0);
        result.Error.Should().Contain("Network error");
    }
}

public class CallExternalApiInputTests
{
    [Fact]
    public void CallExternalApiInput_DefaultMethod_IsGet()
    {
        // Arrange & Act
        var input = new CallExternalApiInput
        {
            Url = "https://example.com"
        };

        // Assert
        input.Method.Should().Be("GET");
    }

    [Fact]
    public void CallExternalApiInput_DefaultTimeout_Is30Seconds()
    {
        // Arrange & Act
        var input = new CallExternalApiInput
        {
            Url = "https://example.com"
        };

        // Assert
        input.TimeoutSeconds.Should().Be(30);
    }
}

public class CallExternalApiOutputTests
{
    [Fact]
    public void CallExternalApiOutput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var output = new CallExternalApiOutput
        {
            Url = "https://example.com",
            StatusCode = 200,
            IsSuccess = true,
            ResponseBody = "{\"data\": \"test\"}"
        };

        // Assert
        output.Url.Should().Be("https://example.com");
        output.StatusCode.Should().Be(200);
        output.IsSuccess.Should().BeTrue();
        output.ResponseBody.Should().Contain("test");
    }
}

public class NotifyActivityTests
{
    private readonly Mock<IHttpClientFactory> _httpClientFactoryMock;
    private readonly Mock<ILogger<NotifyActivity>> _loggerMock;
    private readonly NotifyActivity _activity;

    public NotifyActivityTests()
    {
        _httpClientFactoryMock = new Mock<IHttpClientFactory>();
        _loggerMock = new Mock<ILogger<NotifyActivity>>();

        var httpClient = new HttpClient();
        _httpClientFactoryMock.Setup(x => x.CreateClient(It.IsAny<string>())).Returns(httpClient);

        _activity = new NotifyActivity(_httpClientFactoryMock.Object, _loggerMock.Object);
    }

    [Fact]
    public async Task Run_WithLogChannel_LogsMessage()
    {
        // Arrange
        var input = new NotifyInput
        {
            Channel = NotificationChannel.Log,
            Subject = "Test Subject",
            Message = "Test message content",
            Data = new Dictionary<string, object?>
            {
                ["key1"] = "value1"
            }
        };

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.Success.Should().BeTrue();
        result.Channel.Should().Be(NotificationChannel.Log);
    }

    [Fact]
    public async Task Run_WithLogChannel_SetsCorrectSentAt()
    {
        // Arrange
        var beforeTest = DateTimeOffset.UtcNow;
        var input = new NotifyInput
        {
            Channel = NotificationChannel.Log,
            Subject = "Subject",
            Message = "Message"
        };

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.SentAt.Should().BeOnOrAfter(beforeTest);
        result.SentAt.Should().BeOnOrBefore(DateTimeOffset.UtcNow);
    }
}

public class NotifyInputTests
{
    [Fact]
    public void NotifyInput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var input = new NotifyInput
        {
            Channel = NotificationChannel.Email,
            Subject = "Important",
            Message = "Test message",
            Recipient = "user@example.com"
        };

        // Assert
        input.Channel.Should().Be(NotificationChannel.Email);
        input.Subject.Should().Be("Important");
        input.Message.Should().Be("Test message");
        input.Recipient.Should().Be("user@example.com");
    }
}
