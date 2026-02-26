# Implementation Plan: JSON-Configurable Orchestration System

Based on `opus45design.md` - Azure Durable Functions with Durable Entities and Config-Driven Orchestrators

---

## Project Structure

```
src/
├── Orchestration.Functions/           # Azure Functions App (Isolated Worker)
│   ├── Program.cs                     # Host configuration, DI setup
│   ├── host.json                      # Durable Functions configuration
│   ├── local.settings.json            # Local development settings
│   │
│   ├── Ingress/                       # Event Ingress Layer
│   │   ├── EventIngressFunction.cs    # Service Bus triggered, start-or-route
│   │   └── DeadLetterProcessorFunction.cs
│   │
│   ├── Entities/                      # Durable Entities Layer
│   │   ├── EventAccumulatorEntity.cs  # Event accumulation per device
│   │   └── EventAccumulatorState.cs   # Entity state model
│   │
│   ├── Orchestrators/                 # Orchestration Layer
│   │   ├── WorkflowOrchestrator.cs    # Config-driven workflow interpreter
│   │   └── MonitorOrchestrator.cs     # Stuck workflow detection
│   │
│   ├── Activities/                    # Activity Functions Layer
│   │   ├── Registry/
│   │   │   ├── IActivityRegistry.cs
│   │   │   └── ActivityRegistry.cs
│   │   ├── Database/
│   │   │   ├── CreateRecordActivity.cs
│   │   │   ├── UpdateRecordActivity.cs
│   │   │   └── CompensateRecordActivity.cs
│   │   ├── Entity/
│   │   │   ├── GetEntityEventsActivity.cs
│   │   │   └── ClearEntityActivity.cs
│   │   ├── External/
│   │   │   ├── CallExternalApiActivity.cs
│   │   │   └── NotifyActivity.cs
│   │   └── Workflow/
│   │       ├── LoadWorkflowDefinitionActivity.cs
│   │       └── ValidateWorkflowActivity.cs
│   │
│   └── Http/                          # HTTP Triggers (Management API)
│       ├── StartWorkflowFunction.cs
│       ├── GetWorkflowStatusFunction.cs
│       └── RaiseEventFunction.cs
│
├── Orchestration.Core/                # Shared Domain Logic
│   ├── Workflow/
│   │   ├── WorkflowDefinition.cs      # JSON workflow model
│   │   ├── WorkflowState.cs           # Runtime state
│   │   ├── StateTypes/
│   │   │   ├── TaskState.cs
│   │   │   ├── WaitState.cs
│   │   │   ├── ChoiceState.cs
│   │   │   ├── ParallelState.cs
│   │   │   └── CompensationState.cs
│   │   └── Interpreter/
│   │       ├── IWorkflowInterpreter.cs
│   │       ├── WorkflowInterpreter.cs
│   │       └── JsonPathResolver.cs
│   │
│   ├── Models/
│   │   ├── EventData.cs
│   │   ├── AccumulatedEvent.cs
│   │   ├── WorkflowInput.cs
│   │   └── WorkflowResult.cs
│   │
│   └── Contracts/
│       ├── IEventProcessor.cs
│       └── IWorkflowRepository.cs
│
├── Orchestration.Infrastructure/      # External Dependencies
│   ├── Data/
│   │   ├── OrchestrationDbContext.cs
│   │   └── Repositories/
│   │       └── WorkflowRepository.cs
│   │
│   ├── Storage/
│   │   └── WorkflowDefinitionStorage.cs
│   │
│   └── External/
│       └── ExternalApiClient.cs
│
└── Orchestration.Tests/               # Test Projects
    ├── Unit/
    │   ├── Orchestrators/
    │   ├── Activities/
    │   └── Entities/
    └── Integration/
        └── WorkflowIntegrationTests.cs
```

---

## Phase 1: Project Setup & Infrastructure

### 1.1 Create Solution Structure
- [ ] Create solution `Orchestration.sln`
- [ ] Create `Orchestration.Functions` (Azure Functions Isolated Worker, .NET 10)
- [ ] Create `Orchestration.Core` (Class Library, .NET 10)
- [ ] Create `Orchestration.Infrastructure` (Class Library, .NET 10)
- [ ] Create `Orchestration.Tests` (xUnit, .NET 10)

### 1.2 NuGet Packages

**Orchestration.Functions:**
```xml
<PackageReference Include="Microsoft.Azure.Functions.Worker" Version="2.0.0" />
<PackageReference Include="Microsoft.Azure.Functions.Worker.Sdk" Version="2.0.0" />
<PackageReference Include="Microsoft.Azure.Functions.Worker.Extensions.DurableTask" Version="1.2.2" />
<PackageReference Include="Microsoft.Azure.Functions.Worker.Extensions.ServiceBus" Version="5.22.0" />
<PackageReference Include="Microsoft.Azure.Functions.Worker.Extensions.CosmosDB" Version="4.11.0" />
<PackageReference Include="Microsoft.Azure.Functions.Worker.Extensions.Http.AspNetCore" Version="2.0.0" />
<PackageReference Include="Azure.Identity" Version="1.13.1" />
```

**Orchestration.Core:**
```xml
<PackageReference Include="JsonPath.Net" Version="1.1.6" />
<PackageReference Include="System.Text.Json" Version="9.0.0" />
```

**Orchestration.Infrastructure:**
```xml
<PackageReference Include="Microsoft.EntityFrameworkCore.SqlServer" Version="9.0.0" />
<PackageReference Include="Azure.Storage.Blobs" Version="12.23.0" />
```

### 1.3 Configuration Files

**host.json:**
```json
{
  "version": "2.0",
  "logging": {
    "applicationInsights": {
      "samplingSettings": { "isEnabled": true }
    }
  },
  "extensions": {
    "durableTask": {
      "storageProvider": { "type": "AzureStorage" },
      "extendedSessionsEnabled": true,
      "maxConcurrentActivityFunctions": 10,
      "maxConcurrentOrchestratorFunctions": 10,
      "maxConcurrentEntityFunctions": 10
    },
    "serviceBus": {
      "prefetchCount": 10,
      "sessionHandlerOptions": {
        "maxConcurrentSessions": 16,
        "autoComplete": false
      }
    }
  }
}
```

---

## Phase 2: Core Domain Models

### 2.1 Workflow Definition Schema
- [ ] `WorkflowDefinition.cs` - Root workflow model
- [ ] `WorkflowMetadata.cs` - Author, tags, version
- [ ] `WorkflowConfiguration.cs` - Default timeout, retry policy
- [ ] `RetryPolicy.cs` - Max attempts, intervals, backoff

### 2.2 State Type Models
- [ ] `IWorkflowStateDefinition.cs` - Base interface
- [ ] `TaskStateDefinition.cs` - Activity execution
- [ ] `WaitStateDefinition.cs` - Duration/timestamp/externalEvent
- [ ] `ChoiceStateDefinition.cs` - Conditional branching
- [ ] `ParallelStateDefinition.cs` - Concurrent branches
- [ ] `CompensationStateDefinition.cs` - Saga rollback
- [ ] `TerminalStateDefinition.cs` - Succeed/Fail

### 2.3 Runtime Models
- [ ] `WorkflowState.cs` - Current execution state
- [ ] `WorkflowInput.cs` - Orchestration input
- [ ] `WorkflowResult.cs` - Orchestration output
- [ ] `EventData.cs` - Incoming event model
- [ ] `AccumulatedEvent.cs` - Entity stored event

---

## Phase 3: Durable Entities Layer

### 3.1 Event Accumulator Entity
```csharp
// EventAccumulatorEntity.cs
public class EventAccumulatorEntity : TaskEntity<EventAccumulatorState>
{
    public void RecordEvent(EventData eventData);
    public List<AccumulatedEvent> GetEvents();
    public void Clear();
    public EventAccumulatorState GetState();

    [Function(nameof(EventAccumulatorEntity))]
    public static Task RunEntityAsync([EntityTrigger] TaskEntityDispatcher dispatcher);
}
```

**Implementation Tasks:**
- [ ] Implement `RecordEvent` - Append with timestamp
- [ ] Implement `GetEvents` - Return accumulated list
- [ ] Implement `Clear` - Reset state
- [ ] Implement `GetState` - Return full state
- [ ] Add event count limit (prevent unbounded growth)
- [ ] Add last activity timestamp

---

## Phase 4: Event Ingress Layer

### 4.1 Event Ingress Function (Start-or-Route Pattern)
```csharp
// EventIngressFunction.cs
[Function(nameof(EventIngressFunction))]
public async Task Run(
    [ServiceBusTrigger("device-events", IsSessionsEnabled = true)]
    ServiceBusReceivedMessage message,
    ServiceBusMessageActions messageActions,
    [DurableClient] DurableTaskClient client)
```

**Implementation Tasks:**
- [ ] Extract entity ID from session ID
- [ ] Check orchestration status via `GetInstanceAsync`
- [ ] Start new orchestration if none exists or completed/failed
- [ ] Signal entity to accumulate event via `SignalEntityAsync`
- [ ] Raise event to running orchestration via `RaiseEventAsync`
- [ ] Handle message completion/abandonment
- [ ] Implement dead-letter routing on repeated failures

### 4.2 Dead Letter Processor
- [ ] `DeadLetterProcessorFunction.cs` - Monitor DLQ
- [ ] Log failed messages with correlation
- [ ] Alert operations (Application Insights, webhook)
- [ ] Optional: Retry recoverable messages

---

## Phase 5: Activity Functions Layer

### 5.1 Activity Registry
```csharp
public interface IActivityRegistry
{
    void Register<TInput, TOutput>(string name, string description);
    ActivityMetadata? GetMetadata(string name);
    bool Exists(string name);
    IReadOnlyList<ActivityMetadata> GetAll();
}
```

**Implementation Tasks:**
- [ ] Implement in-memory registry
- [ ] Self-registration via attributes
- [ ] Input/output JSON schema validation
- [ ] Runtime activity lookup

### 5.2 Database Activities
- [ ] `CreateRecordActivity.cs` - Insert with idempotency key
- [ ] `UpdateRecordActivity.cs` - Update existing record
- [ ] `GetRecordActivity.cs` - Read record
- [ ] `CompensateCreateRecordActivity.cs` - Rollback creation

### 5.3 Entity Activities
- [ ] `GetEntityEventsActivity.cs` - Call entity.GetEvents()
- [ ] `ClearEntityActivity.cs` - Call entity.Clear()

### 5.4 External API Activities
- [ ] `CallExternalApiActivity.cs` - HTTP client with retry
- [ ] `NotifyActivity.cs` - Send notifications

### 5.5 Workflow Activities
- [ ] `LoadWorkflowDefinitionActivity.cs` - Load from storage
- [ ] `ValidateWorkflowActivity.cs` - Schema validation

### 5.6 Idempotency Pattern (All Activities)
```csharp
// Template for idempotent activity
public async Task<TResult> Run([ActivityTrigger] TInput input)
{
    // 1. Check idempotency key
    var existing = await _repository.GetByKeyAsync(input.IdempotencyKey);
    if (existing is not null) return existing;

    // 2. Execute logic
    var result = await ExecuteAsync(input);

    // 3. Store result
    await _repository.SaveAsync(input.IdempotencyKey, result);
    return result;
}
```

---

## Phase 6: Config-Driven Orchestrator

### 6.1 Workflow Interpreter Engine
```csharp
public interface IWorkflowInterpreter
{
    Task<string?> ExecuteStepAsync(
        TaskOrchestrationContext context,
        WorkflowDefinition definition,
        string currentStep,
        WorkflowState state);
}
```

**Implementation Tasks:**
- [ ] Step type dispatcher (task, wait, choice, parallel, compensation)
- [ ] JSONPath input resolution
- [ ] Output storage in state
- [ ] Transition evaluation
- [ ] Error handling and compensation triggering

### 6.2 JSONPath Resolver
```csharp
public interface IJsonPathResolver
{
    object? Resolve(string path, WorkflowState state);
    void SetValue(string path, object value, WorkflowState state);
}
```

- [ ] Implement JSONPath expression evaluation
- [ ] Support `$.input.*`, `$.state.*`, `$.system.*`
- [ ] Handle missing paths gracefully

### 6.3 Main Orchestrator
```csharp
[Function(nameof(WorkflowOrchestrator))]
public async Task<WorkflowResult> RunOrchestrator(
    [OrchestrationTrigger] TaskOrchestrationContext context)
{
    var input = context.GetInput<WorkflowInput>()!;

    // Load workflow ONCE (determinism)
    var workflowDef = await context.CallActivityAsync<WorkflowDefinition>(
        nameof(LoadWorkflowDefinitionActivity), input.WorkflowType);

    var state = new WorkflowState { Input = input };
    var currentStep = workflowDef.StartAt;

    while (currentStep is not null)
    {
        currentStep = await _interpreter.ExecuteStepAsync(
            context, workflowDef, currentStep, state);
    }

    return new WorkflowResult { Success = true, State = state };
}
```

### 6.4 State Type Handlers

**Task State:**
- [ ] Resolve input via JSONPath
- [ ] Call activity with retry options
- [ ] Store output in state
- [ ] Handle errors → compensation

**Wait State:**
- [ ] Duration wait: `context.CreateTimer()`
- [ ] Timestamp wait: `context.CreateTimer(resolvedTime)`
- [ ] External event: `context.WaitForExternalEvent()` with timeout
- [ ] Timeout handling → timeoutNext state

**Choice State:**
- [ ] Evaluate conditions against state
- [ ] Support operators: equals, notEquals, greaterThan, lessThan, contains
- [ ] Support logical: and, or, not
- [ ] Default transition

**Parallel State:**
- [ ] Execute branches via `Task.WhenAll`
- [ ] Collect results from all branches
- [ ] Handle partial failures

**Compensation State:**
- [ ] Execute steps in defined order
- [ ] Each step is idempotent
- [ ] Continue despite individual step failures

---

## Phase 7: HTTP Management API

### 7.1 Start Workflow Endpoint
```csharp
[Function("StartWorkflow")]
public async Task<HttpResponseData> Run(
    [HttpTrigger(AuthorizationLevel.Function, "post", Route = "workflows")]
    HttpRequestData req,
    [DurableClient] DurableTaskClient client)
```

- [ ] Parse workflow type and input from body
- [ ] Generate instance ID (or use provided)
- [ ] Start orchestration
- [ ] Return check-status response

### 7.2 Get Status Endpoint
```csharp
[Function("GetWorkflowStatus")]
public async Task<HttpResponseData> Run(
    [HttpTrigger(AuthorizationLevel.Function, "get", Route = "workflows/{instanceId}")]
    HttpRequestData req,
    [DurableClient] DurableTaskClient client,
    string instanceId)
```

- [ ] Get orchestration metadata
- [ ] Return status, created time, last updated, custom status

### 7.3 Raise Event Endpoint
```csharp
[Function("RaiseEvent")]
public async Task<HttpResponseData> Run(
    [HttpTrigger(AuthorizationLevel.Function, "post", Route = "workflows/{instanceId}/events/{eventName}")]
    HttpRequestData req,
    [DurableClient] DurableTaskClient client,
    string instanceId,
    string eventName)
```

- [ ] Parse event data from body
- [ ] Raise event to orchestration
- [ ] Return accepted response

---

## Phase 8: Monitoring & Observability

### 8.1 Stuck Workflow Monitor
```csharp
[Function("StuckWorkflowMonitor")]
public async Task Run(
    [TimerTrigger("0 */15 * * * *")] TimerInfo timer,
    [DurableClient] DurableTaskClient client)
```

- [ ] Query running orchestrations
- [ ] Filter by age > threshold (50 hours)
- [ ] Alert operations team
- [ ] Log to Application Insights

### 8.2 Application Insights Integration
- [ ] Configure distributed tracing
- [ ] Custom metrics: orchestration duration, entity event counts
- [ ] Alerts for activity failure rate
- [ ] Dashboard queries

---

## Phase 9: Infrastructure Layer

### 9.1 Database Context
```csharp
public class OrchestrationDbContext : DbContext
{
    public DbSet<OnboardingRecord> OnboardingRecords { get; set; }
    public DbSet<IdempotencyRecord> IdempotencyRecords { get; set; }
}
```

- [ ] Entity configurations
- [ ] Migrations
- [ ] Connection string from Key Vault

### 9.2 Workflow Definition Storage
```csharp
public interface IWorkflowDefinitionStorage
{
    Task<WorkflowDefinition> GetAsync(string workflowType, string? version = null);
    Task SaveAsync(WorkflowDefinition definition);
    Task<IReadOnlyList<string>> ListVersionsAsync(string workflowType);
}
```

- [ ] Blob storage implementation
- [ ] Versioning support
- [ ] Caching layer

---

## Phase 10: Testing

### 10.1 Unit Tests

**Orchestrator Tests:**
- [ ] Happy path workflow execution
- [ ] Timeout handling
- [ ] Compensation on failure
- [ ] Choice state branching

**Activity Tests:**
- [ ] Idempotency verification
- [ ] Error handling
- [ ] Input validation

**Entity Tests:**
- [ ] Event accumulation
- [ ] State persistence
- [ ] Clear operation

### 10.2 Integration Tests (Azurite)
- [ ] Full workflow execution
- [ ] External event handling
- [ ] Entity event accumulation during wait
- [ ] Compensation flow

### 10.3 Load Tests
- [ ] 1000 concurrent workflows
- [ ] 200 events per entity
- [ ] Measure storage throughput
- [ ] Validate partition distribution

---

## Phase 11: Deployment

### 11.1 Azure Resources
- [ ] Function App (Consumption or Premium)
- [ ] Storage Account (for Durable Functions)
- [ ] Service Bus namespace with session-enabled queue
- [ ] SQL Database
- [ ] Key Vault
- [ ] Application Insights
- [ ] (Optional) Cosmos DB

### 11.2 Infrastructure as Code
- [ ] Bicep/ARM templates
- [ ] Environment-specific parameters
- [ ] Managed Identity configuration

### 11.3 CI/CD Pipeline
- [ ] Build and test
- [ ] Deploy infrastructure
- [ ] Deploy Function App
- [ ] Smoke tests

---

## Implementation Order (Recommended)

| Phase | Description | Dependencies | Estimated Effort |
|-------|-------------|--------------|------------------|
| 1 | Project Setup | None | 2-4 hours |
| 2 | Core Domain Models | Phase 1 | 4-6 hours |
| 3 | Durable Entities | Phase 2 | 4-6 hours |
| 4 | Event Ingress | Phase 3 | 4-6 hours |
| 5 | Activity Functions | Phase 2 | 8-12 hours |
| 6 | Config-Driven Orchestrator | Phase 3, 5 | 12-16 hours |
| 7 | HTTP Management API | Phase 6 | 4-6 hours |
| 8 | Monitoring | Phase 6 | 4-6 hours |
| 9 | Infrastructure | Phase 5 | 6-8 hours |
| 10 | Testing | Phase 1-9 | 12-16 hours |
| 11 | Deployment | Phase 10 | 4-8 hours |

---

## Key Implementation Notes

### Determinism Rules (Critical)
```csharp
// NEVER in orchestrators:
DateTime.Now / DateTime.UtcNow  // Use context.CurrentUtcDateTime
Guid.NewGuid()                  // Use context.NewGuid()
Random                          // Not allowed
Environment variables           // Load via activity
File I/O                        // Move to activity
HTTP calls                      // Move to activity
Database access                 // Move to activity
```

### Idempotency Pattern (All Activities)
```csharp
// Every activity that modifies state must:
1. Accept idempotency key in input
2. Check for existing result before execution
3. Store result after execution
4. Return same result on retry
```

### Compensation Pairs
| Activity | Compensating Activity |
|----------|----------------------|
| CreateOnboardingRecord | RollbackOnboardingRecord |
| ProcessDeviceEvents | UndoDeviceProcessing |
| UpdateExternalSystem | RevertExternalSystem |

### Service Bus Session Configuration
```csharp
// Session ID = Entity ID = Orchestration Instance ID
// Guarantees FIFO per entity, parallel across entities
[ServiceBusTrigger("queue", IsSessionsEnabled = true)]
```
