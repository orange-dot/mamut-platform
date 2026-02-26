# Azure Durable Functions Orchestration System

## Architecture Overview

A JSON-configurable orchestration system for long-running stateful workflows using Azure Durable Functions.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           ORCHESTRATION SYSTEM                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐     ┌──────────────────────────────────────────────────┐  │
│  │ Service Bus  │────▶│  Event Ingress Function                          │  │
│  │ (Sessions)   │     │  - Routes to correct entity/orchestration        │  │
│  └──────────────┘     │  - Starts orchestrations if none exists          │  │
│                       └─────────────────────┬────────────────────────────┘  │
│                                             │                                │
│                                             ▼                                │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     DURABLE ENTITIES LAYER                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │   │
│  │  │ Device-001  │  │ Device-002  │  │ Device-NNN  │  (1 per entity)  │   │
│  │  │ Event Store │  │ Event Store │  │ Event Store │                  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │   │
│  └──────────────────────────────────┬───────────────────────────────────┘   │
│                                     │                                        │
│                                     ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                  CONFIG-DRIVEN ORCHESTRATOR                          │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │  Workflow Interpreter Engine                                 │    │   │
│  │  │  - Reads JSON workflow definition                           │    │   │
│  │  │  - Executes steps dynamically                               │    │   │
│  │  │  - Manages state transitions                                │    │   │
│  │  │  - Handles external event waits                             │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  └───────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     ACTIVITY FUNCTIONS LAYER                         │   │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐        │   │
│  │  │ DB Writer  │ │ API Caller │ │ Notifier   │ │ Validator  │  ...   │   │
│  │  └────────────┘ └────────────┘ └────────────┘ └────────────┘        │   │
│  └──────────────────────────────────┬───────────────────────────────────┘   │
│                                     │                                        │
│                                     ▼                                        │
│                        ┌─────────────────────────┐                          │
│                        │     SQL Database        │                          │
│                        │  (Shared with external) │                          │
│                        └─────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Runtime | .NET 10 / C# 13 | Latest LTS, modern language features |
| Functions | Azure Functions Isolated Worker | Better dependency injection, performance |
| Orchestration | Azure Durable Functions | Automatic state persistence, serverless scaling |
| Event Accumulation | Durable Entities | Concurrent-safe, separate from orchestration history |
| Message Ingestion | Service Bus with Sessions | FIFO per entity, parallel across entities |
| Workflow Config | JSON Schema (ASL-inspired) | Industry-proven patterns, non-developer readable |
| State Persistence | Azure Storage | Managed by framework |
| Shared Database | SQL Server | Transactional consistency with external processes |

---

## Core Components

### 1. Event Ingress Function

**Purpose**: Entry point for all incoming events via Service Bus.

**Pattern**: Start-or-Route
- Check if orchestration exists for entity
- Start new orchestration if none exists or previous completed
- Route events to Durable Entity for accumulation
- Raise events to waiting orchestrations

**Key Features**:
- Service Bus sessions (SessionId = EntityId) for FIFO ordering
- 1000 concurrent sessions supported
- Dead letter handling for unroutable messages

### 2. Durable Entities (Event Accumulation)

**Purpose**: Buffer events during long waiting periods (12-48 hours).

**Operations**:
| Operation | Description |
|-----------|-------------|
| `RecordEvent` | Append event with timestamp |
| `GetEvents` | Return all accumulated events |
| `Clear` | Reset event list after processing |
| `GetState` | Return full state including event count |

**Why Entities over Orchestrator-based accumulation**:
- Concurrent-safe without race conditions
- No replay overhead (orchestrator history bloat)
- Separate accumulation state from orchestration logic

### 3. Config-Driven Orchestrator

**Purpose**: Execute JSON workflow definitions dynamically.

**Execution Model**:
1. Load workflow definition ONCE at start (determinism)
2. Initialize state with entity ID and trigger event
3. Execute steps based on type (task, wait, choice, parallel, compensation)
4. Resolve inputs via JSONPath expressions
5. Handle external events with configurable timeouts
6. Execute compensation on errors

**State Types**:
| Type | Description |
|------|-------------|
| `task` | Execute activity function |
| `wait` | Pause for duration/timestamp/external event |
| `choice` | Conditional branching |
| `parallel` | Concurrent branch execution |
| `compensation` | Saga rollback steps |
| `succeed/fail` | Terminal states |

### 4. Activity Function Registry

**Purpose**: Building blocks for actual work execution.

**Categories**:
- **Database**: Read/write/update SQL records
- **External API**: HTTP calls with retry logic
- **Messaging**: Notifications, event publishing
- **Polling**: External system status checks
- **Entity**: Interact with Durable Entities

**Requirements**:
- Self-registration with name and schemas
- All activities must be **idempotent**
- All I/O happens exclusively in activities

---

## JSON Workflow Definition Schema

```json
{
  "$schema": "workflow-schema-v1.json",
  "id": "device-onboarding-workflow",
  "version": "1.0.0",
  "description": "Handles device onboarding with external dependency wait",

  "input": {
    "type": "object",
    "required": ["entityId", "entityType"],
    "properties": {
      "entityId": { "type": "string" },
      "entityType": { "type": "string" }
    }
  },

  "configuration": {
    "defaultTimeout": "PT48H",
    "retryPolicy": {
      "maxAttempts": 3,
      "initialInterval": "PT5S",
      "backoffCoefficient": 2.0
    }
  },

  "states": {
    "Initialize": {
      "type": "task",
      "activity": "CreateOnboardingRecord",
      "input": {
        "entityId": "$.input.entityId",
        "timestamp": "$.system.currentTime"
      },
      "output": "$.state.recordId",
      "next": "WaitForExternalProcess"
    },

    "WaitForExternalProcess": {
      "type": "wait",
      "waitType": "externalEvent",
      "eventName": "ExternalProcessComplete",
      "timeout": "PT48H",
      "timeoutNext": "HandleTimeout",
      "next": "CollectAccumulatedEvents"
    },

    "CollectAccumulatedEvents": {
      "type": "task",
      "activity": "GetEntityEvents",
      "input": { "entityId": "$.input.entityId" },
      "output": "$.state.events",
      "next": "ProcessEventBatch"
    },

    "ProcessEventBatch": {
      "type": "task",
      "activity": "ProcessDeviceEvents",
      "input": {
        "recordId": "$.state.recordId",
        "events": "$.state.events"
      },
      "onError": "CompensateOnboarding",
      "next": "FinalizeOnboarding"
    },

    "CompensateOnboarding": {
      "type": "compensation",
      "steps": [
        { "activity": "RollbackOnboardingRecord", "input": { "recordId": "$.state.recordId" } },
        { "activity": "NotifyOnboardingFailure", "input": { "entityId": "$.input.entityId" } }
      ],
      "next": "Failed"
    },

    "Success": { "type": "succeed" },
    "Failed": { "type": "fail" }
  },

  "startAt": "Initialize"
}
```

---

## Critical Implementation Rules

### Determinism in Orchestrators

```csharp
// ❌ NEVER use in orchestrators:
DateTime.Now           // Use context.CurrentUtcDateTime
DateTime.UtcNow        // Use context.CurrentUtcDateTime
Guid.NewGuid()         // Use context.NewGuid()
new Random()           // Not allowed
File.ReadAllText()     // Move to activity
HttpClient.GetAsync()  // Move to activity
Environment.GetEnvironmentVariable()  // Load via activity
```

### Idempotent Activities

```csharp
[Function(nameof(ProcessOrderActivity))]
public async Task<OrderResult> Run([ActivityTrigger] ProcessOrderInput input)
{
    // 1. Check idempotency key
    var existing = await _repository.GetByKeyAsync(input.IdempotencyKey);
    if (existing is not null) return existing;

    // 2. Execute logic
    var result = await ProcessOrderAsync(input);

    // 3. Store result
    await _repository.SaveAsync(input.IdempotencyKey, result);
    return result;
}
```

### Compensation Pairs (Saga Pattern)

| Activity | Compensating Activity |
|----------|----------------------|
| CreateOnboardingRecord | RollbackOnboardingRecord |
| ProcessDeviceEvents | UndoDeviceProcessing |
| UpdateExternalSystem | RevertExternalSystemUpdate |

### Replay-Safe Logging

```csharp
// ❌ Bad - logs on every replay
_logger.LogInformation("Step completed");

// ✅ Good - logs only on first execution
var logger = context.CreateReplaySafeLogger<MyOrchestrator>();
logger.LogInformation("Step completed");
```

---

## Service Bus Integration

### Session-Based Correlation

```csharp
[Function(nameof(EventIngressFunction))]
public async Task Run(
    [ServiceBusTrigger("device-events", IsSessionsEnabled = true)]
    ServiceBusReceivedMessage message,
    ServiceBusMessageActions messageActions,
    [DurableClient] DurableTaskClient client)
{
    var entityId = message.SessionId; // Device ID from session
    // Session guarantees FIFO per entity
}
```

### Configuration (host.json)

```json
{
  "extensions": {
    "durableTask": {
      "extendedSessionsEnabled": true,
      "maxConcurrentActivityFunctions": 10,
      "maxConcurrentOrchestratorFunctions": 10
    },
    "serviceBus": {
      "sessionHandlerOptions": {
        "maxConcurrentSessions": 16
      }
    }
  }
}
```

---

## Scaling Considerations

### Target Scale
- 200-1000 concurrent workflows
- 2-200 events per entity over 12-48 hours
- ~200,000 entity operations during waiting periods

### Azure Storage Bottleneck
- Enable `extendedSessionsEnabled` to reduce round-trips
- Consider Premium storage for high-volume scenarios
- Dedicated storage account per task hub if needed

### Entity Partitioning
- Entity IDs distribute automatically across partitions
- Avoid ID patterns that cluster (same prefix)
- Device ID-based naming ensures even distribution

### Control Queue Management
- Default 4 partitions sufficient for 1000 orchestrations
- Monitor queue depth for processing backlog

---

## Error Handling

### Activity Retry Policy

```csharp
var options = new TaskOptions(new TaskRetryOptions(
    new RetryPolicy(
        maxNumberOfAttempts: 3,
        firstRetryInterval: TimeSpan.FromSeconds(5),
        backoffCoefficient: 2.0)));

await context.CallActivityAsync("UnreliableActivity", input, options);
```

### Timeout Escalation
1. Log detailed state for investigation
2. Notify operations team via configured channel
3. Optionally attempt recovery
4. If unrecoverable, execute compensation and mark workflow failed

---

## Versioning Strategy

### Workflow Definition Versioning
- Store with semantic versioning
- Capture version at orchestration start
- Never modify definitions for in-flight instances
- Minor version: non-breaking changes
- Major version: breaking changes (side-by-side deployment)

### Safe Deployment Process
1. Deploy new activity functions first (backward compatible)
2. Deploy orchestrator with version-aware branching
3. Update default version after confirming no issues
4. Monitor for in-flight instances on old version
5. Remove old code paths after all old instances complete

---

## Security

### Secrets Management
- All sensitive config in Azure Key Vault
- Reference via Key Vault references in App Configuration
- Never embed secrets in workflow definitions

### Workflow Definition Validation
- JSON schema validation for structural correctness
- Activity name validation against registry
- Reachability analysis for all states
- Timeout value sanity checks

---

## Monitoring

### Key Metrics
- Orchestration duration
- Events accumulated per entity
- Activity failure rate
- Control queue depth

### Stuck Workflow Detection
- Query orchestrations in "Running" status beyond threshold
- Device onboarding: alert if running > 50 hours (48h wait + buffer)

---

## Testing Strategy

| Test Type | Focus |
|-----------|-------|
| Unit (Orchestrators) | Activity call sequence, condition evaluation, compensation |
| Unit (Activities) | Idempotency, error handling, input validation |
| Integration (Azurite) | Full workflow, event accumulation, compensation flow |
| Load | 1000 concurrent workflows, storage throughput, partition distribution |
