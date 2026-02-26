# Cosmos DB Sink Configuration Options

This document describes the environment variables for configuring the Cosmos DB Sink with batching, retry logic, and exactly-once semantics.

## Core Configuration

### Required Variables
- `COSMOS_URI` - Cosmos DB endpoint URI (e.g., https://your-account.documents.azure.com:443/)
- `COSMOS_KEY` - Cosmos DB primary or secondary key
- `COSMOS_DB` - Database name (default: cosmosflink-demo)
- `COSMOS_SOURCE_CONTAINER` - Source container name (default: events-source)
- `COSMOS_SINK_CONTAINER` - Sink container name (default: events-processed)

### Cosmos DB Sink Performance Configuration

#### Batching Options
- `SINK_BATCH_SIZE` - Number of documents to batch before writing (default: 25)
- `SINK_BATCH_TIMEOUT_MS` - Maximum time to wait before flushing batch in milliseconds (default: 5000)
- `SINK_BULK_THRESHOLD` - Number of operations in bulk execution batch (default: 100)
- `SINK_BULK_TIMEOUT_MS` - Timeout for bulk execution in milliseconds (default: 30000)

#### Connection and Concurrency
- `COSMOS_MAX_CONCURRENCY` - Maximum concurrent connections (default: 10)
- `SINK_MAX_INFLIGHT_REQUESTS` - Maximum in-flight requests per connection (default: 5)
- `COSMOS_REQUEST_TIMEOUT_MS` - Request timeout in milliseconds (default: 10000)

#### Retry and Resilience
- `COSMOS_MAX_RETRY_ATTEMPTS` - Maximum retry attempts for failed operations (default: 3)
- `COSMOS_BACKOFF_MS` - Base backoff time between retries in milliseconds (default: 1000)

#### Consistency and Performance
- `SINK_CONSISTENCY_LEVEL` - Cosmos DB consistency level (default: EVENTUAL)
  - Valid values: STRONG, BOUNDED_STALENESS, SESSION, CONSISTENT_PREFIX, EVENTUAL
- `SINK_ENABLE_CONTENT_RESPONSE` - Enable content in response for writes (default: false)

## Example Environment Configuration

```bash
# Core Cosmos DB settings
COSMOS_URI=https://your-cosmos-account.documents.azure.com:443/
COSMOS_KEY=your-cosmos-primary-key
COSMOS_DB=cosmosflink-demo
COSMOS_SOURCE_CONTAINER=events-source
COSMOS_SINK_CONTAINER=events-processed

# Sink performance tuning
SINK_BATCH_SIZE=50
SINK_BATCH_TIMEOUT_MS=3000
COSMOS_MAX_CONCURRENCY=20
SINK_MAX_INFLIGHT_REQUESTS=10

# Retry configuration
COSMOS_MAX_RETRY_ATTEMPTS=5
COSMOS_BACKOFF_MS=500

# Consistency requirements
SINK_CONSISTENCY_LEVEL=SESSION
SINK_ENABLE_CONTENT_RESPONSE=false
```

## Performance Tuning Guidelines

### For High Throughput
- Increase `SINK_BATCH_SIZE` to 100-500
- Increase `COSMOS_MAX_CONCURRENCY` to 50-100
- Set `SINK_CONSISTENCY_LEVEL` to EVENTUAL
- Keep `SINK_ENABLE_CONTENT_RESPONSE=false`

### For Low Latency
- Decrease `SINK_BATCH_TIMEOUT_MS` to 1000-2000
- Decrease `SINK_BATCH_SIZE` to 10-25
- Set `SINK_CONSISTENCY_LEVEL` to SESSION

### For High Reliability
- Increase `COSMOS_MAX_RETRY_ATTEMPTS` to 5-10
- Increase `COSMOS_BACKOFF_MS` to 2000-5000
- Set `SINK_CONSISTENCY_LEVEL` to STRONG or BOUNDED_STALENESS

## Exactly-Once Semantics

The sink implements exactly-once delivery guarantees through:
- **Two-phase commit protocol** - Coordinates with Flink checkpointing
- **Idempotent writes** - Uses upsert operations with document IDs
- **Transaction coordination** - Ensures all parallel writers commit or rollback together
- **State recovery** - Resumes from last committed checkpoint on failure

## Monitoring and Metrics

The sink provides detailed logging for:
- Documents written per batch
- Retry attempts and failures
- Batch processing times
- Connection pool utilization
- Error rates and patterns

Enable DEBUG logging for detailed performance insights:
```bash
# Add to Flink configuration
rootLogger.level = DEBUG
logger.cosmos.name = com.cosmos.flink.sink
logger.cosmos.level = DEBUG
```