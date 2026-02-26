# JSON-configurable orchestration system using Azure Durable Functions

**Bottom Line Up Front:** Azure Durable Functions provides the optimal foundation for a general-purpose, JSON-configurable orchestration system handling long-running stateful workflows. The combination of automatic state persistence, serverless scaling, and native support for external events and long waits (12-48+ hours) makes it ideal for scenarios like device onboarding. Akka.NET offers superior control for complex domain logic but introduces significant operational overhead that isn't justified for this use case. The recommended architecture uses **Durable Entities for event accumulation** combined with **config-driven orchestrators** that interpret JSON workflow definitions at runtime.

---

## Architecture overview and technology decision

The system architecture centers on Azure Durable Functions as the orchestration engine, with Service Bus providing reliable event ingestion and SQL Server serving as the shared state database. This design prioritizes **operational simplicity** over raw performance, leveraging Azure's managed services to minimize infrastructure burden while supporting the required scale of 200-1000 concurrent workflows.

### Why Durable Functions over Akka.NET

The decision favors Durable Functions based on four critical factors. First, **automatic state persistence** eliminates the need to manage event sourcing journals, snapshots, and recovery logic—Durable Functions checkpoints execution state to Azure Storage transparently. Second, **resource efficiency during long waits** means the system incurs no compute costs during 12-48 hour waiting periods; orchestrator instances unload from memory and timers are implemented as storage queue messages with delayed visibility. Third, **operational simplicity** removes the need to manage cluster infrastructure, node discovery, shard coordination, or persistence database connections. Fourth, **native external event support** provides built-in correlation of incoming events to waiting orchestration instances via `RaiseEventAsync`.

Akka.NET would be preferable if the system required sub-millisecond response times, complex supervision hierarchies for fine-grained fault tolerance, or deployment across multiple cloud providers. For the specified requirements—hundreds of concurrent workflows with event accumulation over hours—Durable Functions delivers equivalent functionality with substantially lower operational complexity.

### High-level component architecture

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

## Component responsibilities and design

### Event ingress function

This Service Bus-triggered function serves as the entry point for all incoming events. It implements the **start-or-route pattern**: checking whether an orchestration exists for the entity, starting one if not, and routing events to the appropriate Durable Entity for accumulation.

The function uses **Service Bus sessions** with session ID equal to the device/entity ID, guaranteeing FIFO ordering per entity while enabling parallel processing across different entities. This design handles the scale requirement efficiently—1000 devices means 1000 independent sessions processable in parallel.

**Key responsibilities:**
- Extract entity ID from message (session ID or correlation property)
- Check orchestration status via `GetStatusAsync`
- Start new orchestration if none exists or previous completed
- Signal Durable Entity to accumulate the event
- Raise external event to orchestration if waiting for specific event type
- Dead-letter messages that cannot be routed after retries

### Durable entities for event accumulation

Each entity (device) has a corresponding Durable Entity that accumulates events during waiting periods. This design solves the core requirement of buffering **2 to hundreds of events** per device over 12-48 hours before processing.

**Why Durable Entities over orchestrator-based accumulation:** Entities provide concurrent-safe state updates without race conditions—multiple events arriving simultaneously are serialized automatically. The orchestrator would need to use `WaitForExternalEvent` in a loop, but each event adds to the execution history, causing replay overhead. Entities keep accumulation state separate from orchestration logic, preventing history bloat.

**Entity operations:**
- `recordEvent`: Append event to accumulated list with timestamp
- `getEvents`: Return all accumulated events for batch processing
- `clear`: Reset event list after successful processing
- `getState`: Return current state including event count and last activity time

### Config-driven orchestrator (workflow interpreter)

The orchestrator reads a JSON workflow definition and executes steps dynamically. This is the core innovation enabling new workflows without code changes. The orchestrator acts as an **interpreter** for a workflow DSL rather than containing hard-coded business logic.

**Execution model:**

1. Load workflow definition from configuration (Azure App Configuration, Blob Storage, or database)
2. Initialize workflow state with entity ID and trigger event data
3. For each step in the workflow definition:
   - Resolve input parameters from current state using JSONPath expressions
   - Call the activity function specified by step configuration
   - Store result in state under configured output key
   - Evaluate transition conditions to determine next step
4. Handle external event waits with configurable timeouts
5. On completion or error, execute configured cleanup/compensation steps

**Determinism requirements:** The orchestrator must remain deterministic despite being config-driven. The workflow definition itself must be immutable for a given orchestration instance—load it once at start and store as part of orchestration input. Never reload configuration mid-execution.

### Activity function registry

Activities are the building blocks that perform actual work—database operations, API calls, notifications, validations. The system maintains a registry of available activities with their input/output schemas, enabling JSON workflow definitions to reference them by name.

**Registry design:**
- Activities self-register via attributes or configuration
- Each activity has: unique name, description, input JSON schema, output JSON schema
- Validation occurs at workflow deployment time, not runtime
- Activities must be **idempotent**—safe to retry on failure
- All I/O operations (database, HTTP, messaging) happen exclusively in activities

**Core activity categories:**
- **Database activities:** Read/write/update records in shared SQL database
- **External API activities:** Call third-party services with retry logic
- **Messaging activities:** Send notifications, publish events
- **Polling activities:** Check external system status for monitor pattern
- **Entity activities:** Interact with Durable Entities (get accumulated events, clear state)

---

## JSON workflow definition schema

The workflow schema draws inspiration from AWS Step Functions' Amazon States Language and the CNCF Serverless Workflow specification, adapted for Azure Durable Functions' specific capabilities.

### Schema design principles

The schema must express sequential execution, conditional branching, parallel execution, external event waits, and error handling while remaining simple enough for non-developers to modify. It should support **state machine semantics** for complex approval workflows and **sequential semantics** for linear processing pipelines.

### Conceptual schema structure

```json
{
  "$schema": "workflow-schema-v1.json",
  "id": "device-onboarding-workflow",
  "version": "1.0.0",
  "description": "Handles device onboarding with external dependency wait",
  "metadata": {
    "author": "platform-team",
    "tags": ["device", "onboarding"]
  },
  
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
      "input": {
        "entityId": "$.input.entityId"
      },
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
    
    "FinalizeOnboarding": {
      "type": "task",
      "activity": "CompleteOnboardingRecord",
      "input": {
        "recordId": "$.state.recordId",
        "status": "completed"
      },
      "next": "Success"
    },
    
    "HandleTimeout": {
      "type": "task",
      "activity": "EscalateTimeout",
      "input": {
        "entityId": "$.input.entityId",
        "waitedHours": 48
      },
      "next": "Failed"
    },
    
    "CompensateOnboarding": {
      "type": "compensation",
      "steps": [
        {
          "activity": "RollbackOnboardingRecord",
          "input": { "recordId": "$.state.recordId" }
        },
        {
          "activity": "NotifyOnboardingFailure",
          "input": { "entityId": "$.input.entityId" }
        }
      ],
      "next": "Failed"
    },
    
    "Success": { "type": "succeed" },
    "Failed": { "type": "fail" }
  },
  
  "startAt": "Initialize"
}
```

### State types

**Task state:** Executes a registered activity function. Input is resolved from workflow state using JSONPath. Output is stored at specified path. Supports retry configuration and error handling.

**Wait state:** Pauses execution. Three wait types supported:
- `duration`: Wait for specified time period (ISO 8601 duration)
- `timestamp`: Wait until specific time (resolved from state)
- `externalEvent`: Wait for named external event with timeout

**Choice state:** Conditional branching based on state evaluation. Conditions use JSONPath expressions evaluated against current state. Supports comparison operators (equals, greater than, contains, etc.) and logical operators (and, or, not).

**Parallel state:** Execute multiple branches concurrently. Each branch is a sequence of states. All branches must complete before proceeding. Supports partial failure handling.

**Compensation state:** Executes compensating actions in reverse order when errors occur. Part of saga pattern implementation.

**Succeed/Fail states:** Terminal states indicating workflow completion status.

---

## Data flow for device onboarding reference case

The device onboarding scenario illustrates all system capabilities: event accumulation during external process wait, conditional processing based on accumulated data, and coordination with shared database state.

### Phase 1: Workflow initiation

When the first event arrives for a new device, the Event Ingress Function detects no existing orchestration. It starts a new workflow instance with the device ID as both the orchestration instance ID and the Durable Entity ID—ensuring permanent correlation. The orchestrator loads the configured workflow definition and executes the Initialize state, creating a record in the shared SQL database with status "pending."

### Phase 2: Accumulation during external wait (12-48 hours)

The orchestrator enters WaitForExternalProcess state, calling `WaitForExternalEvent("ExternalProcessComplete")` with a 48-hour timeout. **The orchestrator is now unloaded from memory**—no compute resources consumed during the wait.

Meanwhile, events continue arriving via Service Bus. The Event Ingress Function routes each event to the device's Durable Entity via `SignalEntityAsync`. The entity appends each event to its accumulated list. At 200 events per device across 1000 devices, the system handles **200,000 entity operations** during the waiting period—well within Durable Functions' capacity.

The external process (running independently, updating the shared SQL database) completes its work. An external trigger—either polling by a monitor orchestration or webhook callback—calls `RaiseEventAsync` to signal completion to the waiting orchestration.

### Phase 3: Event processing and completion

The orchestrator wakes and proceeds to CollectAccumulatedEvents state. It calls the Durable Entity to retrieve all accumulated events—potentially hundreds per device. The ProcessEventBatch activity processes these events against the now-complete external data in the SQL database.

On success, FinalizeOnboarding updates the database record to "completed" status. The entity receives a clear signal to reset its state for potential future workflows.

### Phase 4: Error handling and compensation

If ProcessEventBatch fails, the workflow transitions to CompensateOnboarding. The compensation state executes rollback activities in reverse order: first reverting database state, then sending failure notifications. The workflow terminates in Failed state with full audit trail preserved in orchestration history.

---

## Service Bus integration patterns

### Session-based correlation

Service Bus sessions provide the critical guarantee of **FIFO processing per device** while enabling **parallel processing across devices**. Configure the queue with sessions enabled and require senders to set SessionId equal to the device ID.

The Event Ingress Function uses `IsSessionsEnabled = true` on its trigger, causing Azure Functions to acquire exclusive session locks. All messages for device-123 process through a single function instance in order, while messages for device-456 can process simultaneously on a different instance.

**Scaling characteristics:** With 1000 concurrent devices, the system can theoretically process 1000 sessions in parallel. The `maxConcurrentSessions` setting in host.json controls actual parallelism per function instance. A typical configuration might use 8-16 concurrent sessions per instance, with Azure scaling instances based on session backlog.

### Handling orphaned events

Events may arrive before an orchestration exists (first event for a device) or after completion (late-arriving events). The start-or-route pattern handles these cases:

- **No orchestration exists:** Start new orchestration with the triggering event as input
- **Orchestration completed/failed:** Optionally start new orchestration or dead-letter based on business rules
- **Orchestration running but not waiting for this event type:** Signal entity to accumulate; orchestration will retrieve later

### Dead letter handling

Implement a separate function monitoring the dead letter queue. Messages land in DLQ when:
- Routing fails repeatedly (instance lookup errors)
- Message format invalid
- Business logic explicitly rejects message

The DLQ processor logs for investigation, alerts operations, and optionally attempts remediation for recoverable scenarios.

---

## Monitoring and observability design

### Application Insights integration

Enable end-to-end distributed tracing by configuring Durable Functions to emit telemetry. Key metrics to track:

- **Orchestration duration:** Alert when workflows exceed expected completion time
- **Events accumulated per entity:** Identify entities with unusual event volumes
- **Activity failure rate:** Detect systematic issues with external dependencies
- **Control queue depth:** Early warning of processing backlog

### Workflow status dashboard

Build a dashboard showing real-time workflow status:
- Count of orchestrations by status (Running, Completed, Failed, Pending)
- Average time in each workflow state
- Events processed per hour
- Entity event accumulation rates

Query patterns using Durable Functions' built-in status APIs or direct Azure Table Storage queries against the orchestration history tables.

### Stuck workflow detection

Long-running workflows require proactive monitoring for stuck instances. Implement a scheduled function that queries for orchestrations in "Running" status beyond expected duration. For the device onboarding case, any workflow running beyond 50 hours (48-hour wait plus processing buffer) warrants investigation.

---

## Versioning and deployment strategy

### Workflow definition versioning

Store workflow definitions with semantic versioning. When an orchestration starts, it captures the current workflow definition version in its input state. **Never modify workflow definitions for in-flight instances**—the captured definition governs execution throughout the workflow's lifetime.

For non-breaking changes (adding optional steps, adjusting timeouts), increment minor version. For breaking changes (reordering steps, changing activity signatures), increment major version and use side-by-side deployment.

### Orchestrator code versioning

Use Durable Functions' built-in orchestration versioning (available in Extension Bundle 4.26.0+). Configure `defaultVersion` in host.json and implement version-aware logic in the orchestrator:

```
if version == "1.0":
    execute original logic path
else if version == "2.0":
    execute updated logic path
```

This ensures in-flight orchestrations continue with their original behavior while new instances use updated logic.

### Safe deployment process

1. Deploy new activity functions first (they must be backward compatible)
2. Deploy orchestrator with version-aware branching
3. Update default version after confirming no issues
4. Monitor for in-flight instances on old version
5. Remove old code paths only after all old instances complete

---

## Scaling and performance considerations

### Azure Storage as bottleneck

Durable Functions uses Azure Storage for all state persistence. For 1000 concurrent workflows each with hundreds of entity operations, the storage account must handle significant throughput. Premium storage or dedicated storage accounts per task hub may be necessary for high-volume scenarios.

**host.json optimizations:**
- Enable `extendedSessionsEnabled` to reduce storage round-trips
- Set appropriate `maxConcurrentActivityFunctions` (10-20 typical)
- Configure `maxConcurrentOrchestratorFunctions` based on workflow complexity

### Entity partitioning

Durable Entities automatically distribute across storage partitions based on entity ID. The device ID-based naming ensures even distribution. Avoid patterns that create hot partitions (e.g., all entities with IDs starting with same prefix).

### Control queue management

Each task hub has multiple control queues distributing orchestration work. For 1000 concurrent orchestrations, the default 4 partitions typically suffice. Monitor queue depth; persistent growth indicates processing cannot keep pace with incoming work.

---

## Error handling and saga pattern implementation

### Activity retry policy

Configure retry policies per activity type in the workflow definition. External API calls typically use exponential backoff with longer intervals; database operations might use shorter intervals with more attempts.

The orchestrator passes retry configuration to `CallActivityWithRetryAsync`, letting the Durable Functions framework handle retry logic transparently.

### Compensation design

Design compensating activities for every activity that modifies external state:

| Activity | Compensating Activity |
|----------|----------------------|
| CreateOnboardingRecord | RollbackOnboardingRecord |
| ProcessDeviceEvents | UndoDeviceProcessing |
| UpdateExternalSystem | RevertExternalSystemUpdate |

Compensating activities must be **idempotent**—calling twice with the same input produces the same result. This handles scenarios where compensation itself fails and must retry.

### Timeout escalation

The 48-hour external process wait includes timeout handling. On timeout:
1. Log detailed state for investigation
2. Notify operations team via configured channel
3. Optionally attempt recovery (retry external process trigger)
4. If unrecoverable, execute compensation and mark workflow failed

---

## Testing strategy

### Unit testing orchestrators

Mock `IDurableOrchestrationContext` to test orchestration logic without Azure Storage. Verify:
- Correct activity call sequence for various workflow paths
- Proper condition evaluation at choice states
- Timeout handling when external events don't arrive
- Compensation execution on activity failures

### Unit testing activities

Test activities as standard functions with mocked dependencies. Focus on:
- Idempotency—calling twice with same input produces same result
- Error handling—proper exceptions for various failure modes
- Input validation—reject malformed data before processing

### Integration testing

Use Azure Storage Emulator (Azurite) for realistic integration tests. Test scenarios:
- Complete workflow execution for happy path
- Event accumulation during wait periods
- External event arrival and processing
- Compensation flow on failures
- Workflow versioning transitions

### Load testing

Simulate expected scale (1000 concurrent workflows) to verify:
- Storage throughput sufficient for event volume
- Entity operations don't create hot partitions
- Control queue depth remains manageable
- End-to-end latency meets requirements

---

## Configuration schema for deployment

The system requires configuration at multiple levels. Environment-specific configuration lives in Azure App Configuration; workflow definitions live in versioned storage (Blob Storage or database).

### System configuration schema concept

```json
{
  "orchestration": {
    "taskHub": "device-orchestration-prod",
    "storageAccount": "deviceorchstorageprod",
    "defaultTimeout": "PT48H",
    "maxConcurrentOrchestrations": 1000
  },
  "serviceBus": {
    "namespace": "device-events-prod",
    "queueName": "device-events",
    "sessionsEnabled": true,
    "maxConcurrentSessions": 16
  },
  "database": {
    "connectionString": "@Microsoft.KeyVault(SecretUri=...)",
    "commandTimeout": 30
  },
  "monitoring": {
    "applicationInsightsKey": "@Microsoft.KeyVault(SecretUri=...)",
    "alertWebhook": "https://alerts.example.com/webhook",
    "stuckWorkflowThresholdHours": 50
  },
  "activities": {
    "externalApi": {
      "baseUrl": "https://api.external.com",
      "timeoutSeconds": 30,
      "retryAttempts": 3
    }
  }
}
```

---

## Security considerations

### Secrets management

Store all sensitive configuration (connection strings, API keys) in Azure Key Vault. Reference via Key Vault references in App Configuration or environment variables—never embed in workflow definitions.

### Entity data isolation

Entity state may contain sensitive event data. Configure storage account with appropriate access controls. Consider encryption at rest beyond Azure's default encryption for highly sensitive data.

### Workflow definition validation

Validate workflow definitions before deployment:
- JSON schema validation for structural correctness
- Activity name validation against registry
- Reachability analysis for all states
- Timeout value sanity checks

Reject definitions that reference non-existent activities or contain unreachable states.

---

## Summary of architectural decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Orchestration engine | Azure Durable Functions | Automatic state persistence, serverless scaling, native long-wait support |
| Event accumulation | Durable Entities | Concurrent-safe, separate from orchestration history, queryable state |
| Message ingestion | Service Bus with sessions | FIFO per entity, parallel across entities, dead letter support |
| Workflow configuration | JSON schema (ASL-inspired) | Industry-proven patterns, tooling ecosystem, readable by non-developers |
| State persistence | Azure Storage (default) | Managed by framework, sufficient for 1000 concurrent workflows |
| Shared database | SQL Server | Existing requirement, transactional consistency with external process |

This architecture provides a foundation for building JSON-configurable orchestration workflows that handle long-running processes with external dependencies. The design is general-purpose—the same system handles device onboarding, document approval workflows, multi-step order processing, or any scenario requiring stateful coordination over extended periods.