# End-to-End Demo Plan: .NET Producer → Flink → Azure Cosmos DB

## Goals

- Demonstrate a full pipeline: Producer (.NET) → Custom Cosmos DB Sink → Azure Cosmos DB (NoSQL API).
- **Additional bi-directional flow**: Custom Cosmos DB Source → Flink → .NET Consumer (for change feed processing).
- **Two parallel implementations**: Java and Scala versions of both custom Cosmos Source and Sink.
- Use a real Azure Cosmos DB account (cost awareness required) and local Docker for all other components.
- Showcase production-grade Cosmos connectors (batching, retries, backpressure, checkpoint alignment, idempotence, metrics).
- Keep run/teardown simple and reproducible via Docker Compose and minimal build steps.
- Compare Java vs Scala approaches for both source and sink Flink connector development.
- **Eliminate intermediary systems**: Direct .NET ↔ Flink ↔ Cosmos DB integration without Kafka.

## Architecture Overview

### Primary Flow (Producer → Source → Sink)
- **Producer**: .NET console app (Dockerized) generates JSON events and writes them directly to Cosmos DB source container.
- **Processing**: Flink cluster (JobManager/TaskManager) in Docker. A Flink job with custom Cosmos source reads from change feed of source container, transforms/enriches, and writes to target container via custom sink.
- **Storage**: Azure Cosmos DB (NoSQL). One database, multiple containers with defined partition keys for different pipeline stages.

### Secondary Flow (Source → Consumer)
- **Source**: Custom Cosmos DB Source reads from processed events container change feed using FLIP-27 architecture.
- **Processing**: Flink job processes Cosmos documents with stateful transformations and sends to .NET consumer via HTTP API.
- **Consumer**: .NET console app (Dockerized) receives processed events from Flink for further processing or display.
- **Observability**: Flink Web UI, application logs; leverage Cosmos metrics in Azure Portal.

## Deliverables

- **docs**
  - This plan and a concise README with run/verify/cleanup steps for both Java and Scala demos.
- **infra**
  - docker-compose.yaml standing up: flink-jobmanager, flink-taskmanager.
  - Optional: a small init script for Flink cluster setup.
  - .env.example capturing config: Cosmos endpoint/key/db/container, partition key path, throughput knobs.
- **producer**
  - .NET 8 console app generating realistic events at a configurable rate; Dockerfile to containerize.
  - Config via env vars: Cosmos endpoint/key for source container, rate, payload shape, direct Cosmos DB SDK integration.
- **consumer**
  - .NET 8 console app receiving processed events from Flink; Dockerfile to containerize.
  - Config via env vars: Flink endpoint, HTTP listener settings, output formatting.
- **flink-job-java**
  - Maven project (Java 17) with Flink pipelines:
    - **Primary Pipeline**: Custom CosmosSource (source container) → transform → Custom CosmosSink (target container)
    - **Secondary Pipeline**: Custom CosmosSource (target container) → transform → HTTP/TCP Sink
  - **Custom Cosmos Source**: FLIP-27 implementation with Change Feed Processor integration
    - `CosmosDBSource<T>` implementing modern Source API
    - `CosmosDBSplitEnumerator` for partition discovery and work distribution
    - `CosmosDBSourceReader` with CFP instances for parallel consumption
    - Support for both change feed (unbounded) and query (bounded) modes
  - **Custom Cosmos Sink**: AsyncSinkBase implementation with bulk operations
  - JAR artifact and submission instructions.
- **flink-job-scala**
  - SBT project (Scala 2.12/3.x) with Flink pipelines:
    - **Primary Pipeline**: Custom CosmosSource (source container) → transform → Custom CosmosSink (target container)
    - **Secondary Pipeline**: Custom CosmosSource (target container) → transform → HTTP/TCP Sink
  - **Custom Cosmos Source**: FLIP-27 implementation with functional programming idioms
    - Pattern matching for document type handling and error conditions
    - For-comprehensions for async CFP operations and state management
    - Case classes for immutable configuration and split state
    - Functional composition for document transformation pipelines
  - **Custom Cosmos Sink**: Type-safe implementation with Scala collections
  - JAR artifact and submission instructions.
  - Demonstrates Scala's functional programming advantages for both source and sink operations.
- **validation**
  - Simple script or instructions to verify documents in Cosmos and offsets/checkpoints in Flink for both implementations.

## Data Model

- Event type: OrderCreated (example). Minimal fields:
  - id: GUID (also used as document id for idempotence)
  - customerId: string (Cosmos partition key)
  - amount: number, currency: string
  - createdAt: ISO timestamp
  - source: string (producer id / region)
- Partition key path: /customerId (ensures scalable writes and aligned reads).
- Upsert policy: Use deterministic id to support idempotence (producer retries, sink retries).

## Azure Cosmos Setup (Real Resource)

- **Prereqs**: Azure subscription + Cosmos DB (NoSQL) account provisioned (region close to your machine for lower latency).
- **Create databases and containers**:
  - **Source Container**: events-source (partition key: /customerId) for producer writes
  - **Target Container**: events-processed (partition key: /region) for Flink sink operations
  - **Output Container**: events-output (partition key: /region) for secondary pipeline processing
  - **Lease Container**: cosmos-leases for Change Feed Processor coordination
  - **Change Feed**: Enable change feed on source and target containers for both pipelines
- **Configuration**:
  - Partition keys: /customerId (primary), /region (secondary)
  - Throughput: autoscale or sufficient RU (start small; raise if throttled)
  - Indexing: default fine; optional composite indexes if needed
  - TTL: optional for automatic cleanup
- **Environment variables**:
  - COSMOS_URI, COSMOS_KEY, COSMOS_DB
  - COSMOS_SOURCE_CONTAINER, COSMOS_TARGET_CONTAINER, COSMOS_OUTPUT_CONTAINER, COSMOS_LEASE_CONTAINER
  - COSMOS_SOURCE_PKEY_PATH (/customerId), COSMOS_TARGET_PKEY_PATH (/region), COSMOS_OUTPUT_PKEY_PATH (/region)
  - CFP_HOST_NAME (unique identifier for this demo instance)
  - CFP_LEASE_PREFIX (shared prefix for coordinating multiple readers)
  - Rate/latency tuning: COSMOS_MAX_CONCURRENCY, COSMOS_MAX_RETRY_ATTEMPTS, COSMOS_BACKOFF_MS

## Custom Cosmos Connectors Design (Production-Grade)

### Custom Cosmos Source Design (Production-Grade FLIP-27 Implementation)

#### Architectural Foundation
- **FLIP-27 Source API**: Modern Source API with master-slave architecture (SplitEnumerator + SourceReader)
- **Change Feed Processor Integration**: Leverages Azure Cosmos DB Change Feed Processor for fault-tolerant consumption
- **Unified Batch/Streaming**: Single API supporting both bounded (query) and unbounded (change feed) modes
- **Split-Based Parallelism**: Each Cosmos DB physical partition becomes a CosmosDBSplit for parallel processing

#### Java Implementation
- **CosmosDBSource**: Main entry point implementing `Source<T, CosmosDBSplit, CosmosDBSourceState>`
- **CosmosDBSplitEnumerator**: Coordinator running on JobManager, discovers partitions via `FeedRange` objects
- **CosmosDBSourceReader**: Parallel workers on TaskManagers, each running Change Feed Processor instances
- **CosmosDBSplit**: Serializable unit of work wrapping `FeedRange` + continuation token state
- **Lease Management**: CFP instances coordinate via lease container with shared `leasePrefix`
- **Fault Tolerance**: Continuation tokens stored in Flink state, automatic split reassignment on failures

#### Scala Implementation
- **Functional Architecture**: Same FLIP-27 pattern but with Scala functional programming idioms
- **Type Safety**: Strong typing for `FeedRange`, continuation tokens, and split assignments
- **Pattern Matching**: Elegant handling of different document types and CFP events
- **For-Comprehensions**: Clean async operations and error handling with `Future`/`Try` compositions
- **Case Classes**: Immutable configuration and state objects with built-in serialization

#### Advanced Features (Both Languages)
- **Partition Discovery**: Automatic detection of Cosmos DB physical partition changes (splits/merges)
- **Dynamic Load Balancing**: CFP handles partition lease distribution across multiple readers
- **Backpressure Handling**: Proper flow control when Flink processing can't keep up
- **Exactly-Once Processing**: Continuation token checkpointing ensures no data loss or duplication
- **Cross-Partition Queries**: Support for SQL queries spanning multiple partitions (batch mode)
- **Watermark Generation**: Automatic event-time watermark emission based on document timestamps

### Custom Cosmos Sink Design

#### Java Implementation
- **API**: Implement Flink Sink v2 (org.apache.flink.api.connector.sink2.Sink) with a Writer using CosmosAsyncClient.
- **Batching**: Aggregate records into batches by partition key where possible; configurable max batch size and flush interval.
- **Idempotence**: Upsert with deterministic id; ensure retry-safe operations.
- **Retries/Backoff**: Handle 429 (RequestRateTooLarge), 5xx with exponential backoff and jitter; use Cosmos SDK retry but also guard with sink-level policies.
- **Concurrency/Throughput**: Control parallelism (writer concurrency), in-flight requests, and per-partition rate; surface metrics.
- **Checkpoint Semantics**: Flush on prepareCommit; buffer writes between checkpoints; avoid duplicates by upsert; optionally implement two-phase commit-like behavior (exactly-once with idempotence).
- **Error Handling**: Classify errors; send poison records to a DLQ Kafka topic or side output; include record metadata in logs.

#### Scala Implementation
- **API**: Same Flink Sink v2 interface but leverage Scala's functional programming features.
- **Case Classes**: Define strongly-typed event models and configuration with case classes.
- **Pattern Matching**: Use pattern matching for error classification and response handling.
- **For-Comprehensions**: Handle async operations and error propagation elegantly with Future/Try compositions.
- **Immutable Collections**: Use Scala collections for batching and buffering operations.
- **Type Safety**: Leverage Scala's type system for compile-time guarantees on partition key extraction and document mapping.
- **Functional Error Handling**: Use Either/Try for error handling instead of exceptions where appropriate.

### Common Features (Both Source and Sink)
- **Mapping**: Configurable mappers for document serialization/deserialization.
- **Metrics**: Comprehensive metrics for throughput, latency, errors, and backpressure.
- **Configuration**: All connector knobs configurable via env/args.

## Flink Jobs

### Java Implementation
- **Primary Pipeline**: 
  - Source: Custom CosmosSource reading from events-source container change feed; event ordering per partition.
  - Transform: Minimal enrich/validate using standard Java streams; drop bad records to side output/DLQ.
  - Sink: Custom CosmosSink writing to events-processed container; parallelism tuned to Cosmos RU and partitions.
- **Secondary Pipeline**:
  - Source: Custom CosmosSource with FLIP-27 architecture reading from events-processed container change feed
    - `CosmosDBSplitEnumerator` discovers partitions via `FeedRange` API
    - `CosmosDBSourceReader` instances run CFP for parallel consumption
    - Continuation tokens stored in Flink state for exactly-once processing
    - Support for both change feed (streaming) and cross-partition queries (batch)
  - Transform: Document enrichment, filtering, and format conversion with CompletableFuture
  - Sink: HTTP/TCP Sink sending processed documents to .NET consumer.
- **Build**: Maven with standard Java 17 features, CompletableFuture for async operations.
- **Testing**: JUnit 5 with Testcontainers for integration tests.

### Scala Implementation
- **Primary Pipeline**:
  - Source: Custom CosmosSource with Scala case classes reading from events-source container change feed.
  - Transform: Leverage Scala's functional collections (map, filter, flatMap) and pattern matching for data validation/enrichment.
  - Sink: Custom CosmosSink writing to events-processed container using Scala's functional programming idioms and type safety.
- **Secondary Pipeline**:
  - Source: Custom CosmosSource with FLIP-27 and functional programming approach reading from events-processed container
    - Pattern matching for handling different CFP events and document types
    - For-comprehensions for clean async operations with Change Feed Processor
    - Case classes for immutable split state and continuation token management
    - Functional error handling with `Try`/`Either` for CFP exceptions
  - Transform: Type-safe transformations using case classes and functional composition
  - Sink: HTTP/TCP Sink with functional error handling and monadic composition.
- **Build**: SBT with Scala 2.12 or 3.x, Future/Try for async operations.
- **Testing**: ScalaTest with property-based testing using ScalaCheck.

### Common Features (Both Implementations)
- **Checkpointing**: Enabled with reasonable interval (e.g., 30s) and timeout; externalized checkpoints for restarts.
- **Serialization**: Ensure stable schema; prefer schema registry if using Avro/Protobuf (optional for demo).
- **Monitoring**: Same Flink metrics and monitoring capabilities regardless of language.
- **Multiple Jobs**: Both sink and source pipelines can run simultaneously or independently.

## Local Docker Stack (Compose)

- Services:
  - flink-jobmanager, flink-taskmanager (apache/flink: matching version)
  - Optional: flink-cluster-initializer (one-shot) for cluster setup
- Networking: single user-defined network; expose Flink UI ports.
- Secrets/Env: Reference .env file for Cosmos configuration; do not bake secrets into images.

## Project Structure

```
cosmosflink/
  DEMO_PLAN.md
  README.md
  infra/
    docker-compose.yaml
    .env.example
  producer/
    src/ Program.cs
    Producer.csproj
    Dockerfile
  flink-job-java/
    pom.xml
    src/main/java/.../FlinkCosmosSinkJob.java
    src/main/java/.../FlinkCosmosSourceJob.java
    src/main/java/.../cosmos/CosmosSource.java
    src/main/java/.../cosmos/CosmosSink.java
    src/main/java/.../cosmos/CosmosDBSplitEnumerator.java
    src/main/java/.../cosmos/CosmosDBSourceReader.java
    src/main/java/.../cosmos/CosmosDBSplit.java
    src/main/java/.../cosmos/CosmosDBSourceState.java
    src/main/java/.../cosmos/CosmosWriter.java
    src/main/java/.../model/OrderEvent.java
    src/test/java/.../cosmos/CosmosSourceTest.java
    src/test/java/.../cosmos/CosmosSinkTest.java
  flink-job-scala/
    build.sbt
    project/
    src/main/scala/.../FlinkCosmosSinkJob.scala
    src/main/scala/.../FlinkCosmosSourceJob.scala
    src/main/scala/.../cosmos/CosmosSource.scala
    src/main/scala/.../cosmos/CosmosSink.scala
    src/main/scala/.../cosmos/CosmosDBSplitEnumerator.scala
    src/main/scala/.../cosmos/CosmosDBSourceReader.scala
    src/main/scala/.../cosmos/CosmosDBSplit.scala
    src/main/scala/.../cosmos/CosmosDBSourceState.scala
    src/main/scala/.../cosmos/CosmosWriter.scala
    src/main/scala/.../model/OrderEvent.scala
    src/test/scala/.../cosmos/CosmosSourceSpec.scala
    src/test/scala/.../cosmos/CosmosSinkSpec.scala
  scripts/ (optional validation/teardown helpers)
```

## Build & Run (High-Level)

1) **Prereqs**: Docker Desktop, Java 17, Maven + SBT, .NET 8 SDK, Azure login + Cosmos account.
2) **Configuration**: Populate infra/.env from .env.example with your Cosmos details (URI, KEY, DB, CONTAINER, PKEY path) and HTTP/TCP endpoint settings.
3) **Infrastructure**: Start local stack with Docker Compose (Flink cluster).
4) **Build Jobs**: 
   - Java: `cd flink-job-java && mvn clean package` (produces primary and secondary pipeline JARs)
   - Scala: `cd flink-job-scala && sbt assembly` (produces primary and secondary pipeline JARs)
5) **Submit Jobs**: Choose either Java or Scala implementation (or run both for comparison):
   - Submit primary pipeline JAR: reads from events-source container, writes to events-processed container
   - Submit secondary pipeline JAR: reads from events-processed container, sends to HTTP/TCP consumer
   - Jobs can run independently or simultaneously for full bi-directional flow
6) **Producer**: Build and run producer container to write events directly to Cosmos events-source container at a controlled rate.
7) **Consumer**: Build and run consumer container to receive processed events from Flink secondary pipeline.
8) **Validation**: Observe Flink UI and verify documents land in Cosmos (Azure Portal/Explorer).

## Language-Specific Benefits Showcase

### Java Implementation Benefits
- **Familiarity**: More developers know Java, easier team adoption.
- **Ecosystem**: Rich Maven ecosystem, extensive tooling, mature IDE support.
- **Performance**: Predictable performance characteristics, well-optimized JVM.
- **Documentation**: Extensive Flink documentation and examples in Java.

### Scala Implementation Benefits
- **Type Safety**: Stronger compile-time guarantees and expressive type system for both source and sink operations.
- **Functional Programming**: Cleaner async/error handling with for-comprehensions and pattern matching in source readers.
- **Conciseness**: Less boilerplate code for change feed processing and document transformations.
- **Data Processing**: Natural fit for data transformations with functional collections and immutable data structures.
- **Pattern Matching**: Elegant handling of different document types and change feed events.

## Validation

- **Functional**: 
  - Primary Pipeline: Count messages produced (to events-source) vs. consumed (from change feed) vs. written (to events-processed); tolerate duplicates due to upsert idempotence.
  - Secondary Pipeline: Verify change feed events from events-processed are properly captured and sent to consumer.
  - Bi-directional: Test round-trip scenarios where secondary pipeline processes primary pipeline outputs.
- **Failure**: 
  - Induce Cosmos 429 by increasing producer rate; observe both producer and Flink pipeline backoff/retries; no message loss.
  - Test change feed processor resilience during Cosmos availability issues.
- **Recovery**: 
  - Restart TaskManager/Job; checkpoint restore; ensure no gaps in change feed processing; re-upserts are safe.
  - Verify change feed continuation tokens are properly restored from checkpoints.
- **Data**: 
  - Spot-check partition key correctness and schema across all containers.
  - Validate change feed event ordering and completeness for both pipelines.

## Configurations & Tuning

- Producer: rate (msg/s), concurrency, payload size, Cosmos DB SDK connection settings.
- Flink: checkpoint interval, parallelism, restart strategy, memory/CPU for TM.
- Cosmos Source: change feed processor settings, continuation token management, preferred regions.
- Cosmos Sink: batch size, flush interval, max in-flight, retry policy, preferred regions, consistency level.
- HTTP/TCP Transport: connection pooling, timeout settings, serialization format (secondary pipeline only).

## Security & Secrets

- Do not commit real Cosmos keys. Use env files kept local or Docker secrets.
- Limit Azure RBAC to least privilege for the Cosmos account.

## Cleanup

- Stop and remove the compose stack; delete local volumes if created.
- Optional: Delete Cosmos DB resources to avoid charges.

## Timeline (Suggested)

- **Day 1**: Infra compose + producer skeleton + Cosmos DB SDK integration for direct writes.
- **Day 2-3**: 
  - Java implementation: Flink job scaffold + Custom Cosmos Source → Log sink; then wire Custom Cosmos Sink.
  - Scala implementation: Same pipeline but using Scala idioms and functional programming.
- **Day 4**: Implement source/sink batching/retries/metrics for both; submit end-to-end; validate; performance comparison.
- **Day 5**: Polish docs, add tests, create comparison analysis between Java vs Scala approaches.

## Next Steps

- **Schema Evolution**: Add schema registry and Avro/Protobuf contracts for both implementations.
- **Monitoring**: Add DLQ handling and a small inspector tool.
- **Infrastructure as Code**: Add IaC for Cosmos (Bicep/Terraform) for repeatable provisioning.
- **CI/CD**: Add CI to build both Java and Scala images and run integration tests.
- **Performance Analysis**: Compare throughput, latency, and resource usage between Java and Scala implementations.
- **Advanced Scala Features**: Explore advanced Scala features like Akka Streams integration or Cats Effect for even more functional approaches.
- **HTTP/TCP Optimization**: Explore connection pooling, async I/O, and serialization optimizations.
