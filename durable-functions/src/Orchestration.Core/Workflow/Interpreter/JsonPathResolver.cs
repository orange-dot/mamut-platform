using System.Text.Json;
using System.Text.Json.Nodes;
using Json.Path;
using Orchestration.Core.Models;

namespace Orchestration.Core.Workflow.Interpreter;

/// <summary>
/// Resolves JSONPath expressions against workflow state.
/// </summary>
public sealed class JsonPathResolver : IJsonPathResolver
{
    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false
    };

    /// <inheritdoc />
    public object? Resolve(string path, WorkflowRuntimeState state)
    {
        if (string.IsNullOrWhiteSpace(path))
            return null;

        var jsonNode = ConvertStateToJsonNode(state);
        var jsonPath = JsonPath.Parse(path);
        var result = jsonPath.Evaluate(jsonNode);

        if (result.Matches == null || result.Matches.Count == 0)
            return null;

        if (result.Matches.Count == 1)
            return ConvertJsonNodeToObject(result.Matches[0].Value);

        return result.Matches.Select(m => ConvertJsonNodeToObject(m.Value)).ToList();
    }

    /// <inheritdoc />
    public T? Resolve<T>(string path, WorkflowRuntimeState state)
    {
        var value = Resolve(path, state);
        if (value == null)
            return default;

        if (value is T typed)
            return typed;

        if (value is JsonElement element)
            return JsonSerializer.Deserialize<T>(element.GetRawText(), SerializerOptions);

        var json = JsonSerializer.Serialize(value, SerializerOptions);
        return JsonSerializer.Deserialize<T>(json, SerializerOptions);
    }

    /// <inheritdoc />
    public void SetValue(string path, object? value, WorkflowRuntimeState state)
    {
        if (string.IsNullOrWhiteSpace(path))
            return;

        var segments = ParsePath(path);
        if (segments.Count == 0)
            return;

        SetValueRecursive(state, segments, value);
    }

    /// <inheritdoc />
    public object? ResolveInput(object? input, WorkflowRuntimeState state)
    {
        if (input == null)
            return null;

        if (input is string str)
        {
            if (str.StartsWith("$."))
                return Resolve(str, state);
            return str;
        }

        if (input is JsonElement element)
            return ResolveJsonElement(element, state);

        if (input is Dictionary<string, object?> dict)
        {
            var result = new Dictionary<string, object?>();
            foreach (var kvp in dict)
            {
                result[kvp.Key] = ResolveInput(kvp.Value, state);
            }
            return result;
        }

        if (input is IList<object?> list)
        {
            return list.Select(item => ResolveInput(item, state)).ToList();
        }

        return input;
    }

    private object? ResolveJsonElement(JsonElement element, WorkflowRuntimeState state)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.String:
                var str = element.GetString();
                if (str?.StartsWith("$.") == true)
                    return Resolve(str, state);
                return str;

            case JsonValueKind.Object:
                var dict = new Dictionary<string, object?>();
                foreach (var prop in element.EnumerateObject())
                {
                    dict[prop.Name] = ResolveJsonElement(prop.Value, state);
                }
                return dict;

            case JsonValueKind.Array:
                return element.EnumerateArray()
                    .Select(e => ResolveJsonElement(e, state))
                    .ToList();

            case JsonValueKind.Number:
                if (element.TryGetInt64(out var longVal))
                    return longVal;
                return element.GetDouble();

            case JsonValueKind.True:
                return true;

            case JsonValueKind.False:
                return false;

            case JsonValueKind.Null:
            case JsonValueKind.Undefined:
            default:
                return null;
        }
    }

    private JsonNode? ConvertStateToJsonNode(WorkflowRuntimeState state)
    {
        // Build the full input object including all WorkflowInput properties
        // Merge Data dictionary into root of input for easy path access like $.input.deviceId
        Dictionary<string, object?>? inputObject = null;
        if (state.Input != null)
        {
            inputObject = new Dictionary<string, object?>
            {
                ["workflowType"] = state.Input.WorkflowType,
                ["version"] = state.Input.Version,
                ["entityId"] = state.Input.EntityId,
                ["idempotencyKey"] = state.Input.IdempotencyKey,
                ["correlationId"] = state.Input.CorrelationId,
                ["data"] = state.Input.Data
            };

            // Merge Data properties into root of input for direct access
            if (state.Input.Data != null)
            {
                foreach (var kvp in state.Input.Data)
                {
                    // Don't overwrite standard properties
                    if (!inputObject.ContainsKey(kvp.Key))
                    {
                        inputObject[kvp.Key] = kvp.Value;
                    }
                }
            }
        }

        var stateObject = new Dictionary<string, object?>
        {
            ["input"] = inputObject,
            ["variables"] = state.Variables,
            ["stepResults"] = state.StepResults,
            ["system"] = new Dictionary<string, object?>
            {
                ["instanceId"] = state.System.InstanceId,
                ["startTime"] = state.System.StartTime,
                ["currentTime"] = state.System.CurrentTime,
                ["retryCount"] = state.System.RetryCount
            }
        };

        var json = JsonSerializer.Serialize(stateObject, SerializerOptions);
        return JsonNode.Parse(json);
    }

    private static object? ConvertJsonNodeToObject(JsonNode? node)
    {
        if (node == null)
            return null;

        return node switch
        {
            JsonValue value => GetJsonValuePrimitive(value),
            JsonObject obj => obj.ToDictionary(
                kvp => kvp.Key,
                kvp => ConvertJsonNodeToObject(kvp.Value)),
            JsonArray arr => arr.Select(ConvertJsonNodeToObject).ToList(),
            _ => null
        };
    }

    private static object? GetJsonValuePrimitive(JsonValue value)
    {
        if (value.TryGetValue<string>(out var str))
            return str;
        if (value.TryGetValue<long>(out var lng))
            return lng;
        if (value.TryGetValue<double>(out var dbl))
            return dbl;
        if (value.TryGetValue<bool>(out var bln))
            return bln;
        if (value.TryGetValue<int>(out var intVal))
            return intVal;
        return value.ToString();
    }

    private List<string> ParsePath(string path)
    {
        var segments = new List<string>();
        var trimmed = path.TrimStart('$', '.');

        foreach (var segment in trimmed.Split('.'))
        {
            if (!string.IsNullOrEmpty(segment))
                segments.Add(segment);
        }

        return segments;
    }

    private void SetValueRecursive(WorkflowRuntimeState state, List<string> segments, object? value)
    {
        if (segments.Count == 0)
            return;

        var root = segments[0];
        var remaining = segments.Skip(1).ToList();

        switch (root)
        {
            case "variables":
                SetInDictionary(state.Variables, remaining, value);
                break;
            case "stepResults":
                SetInDictionary(state.StepResults, remaining, value);
                break;
        }
    }

    private void SetInDictionary(Dictionary<string, object?> dict, List<string> segments, object? value)
    {
        if (segments.Count == 0)
            return;

        if (segments.Count == 1)
        {
            dict[segments[0]] = value;
            return;
        }

        var key = segments[0];
        if (!dict.TryGetValue(key, out var existing) || existing is not Dictionary<string, object?> nested)
        {
            nested = new Dictionary<string, object?>();
            dict[key] = nested;
        }

        SetInDictionary(nested, segments.Skip(1).ToList(), value);
    }
}
