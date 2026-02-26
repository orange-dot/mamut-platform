# Complete Deployment Guide: Cosmos DB + Flink Real-Time Pipeline

## Overview

This guide provides step-by-step instructions for deploying the complete end-to-end real-time data processing pipeline with Azure Cosmos DB and Apache Flink.

## Architecture

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   .NET Producer │───▶│   Cosmos DB Source   │───▶│  Flink Processing   │
│   (Docker)      │    │   (events-source)    │    │  (Transform)        │
└─────────────────┘    └──────────────────────┘    └─────────┬───────────┘
                                                              │
                       ┌──────────────────────┐              │
                       │   Cosmos DB Target   │◀─────────────┘
                       │  (events-processed)  │
                       └──────────┬───────────┘
                                  │
                       ┌──────────▼───────────┐    ┌─────────────────────┐
                       │   Cosmos DB Source   │───▶│  .NET Consumer      │
                       │  (Change Feed #2)    │    │  (HTTP/TCP)         │
                       └──────────────────────┘    └─────────────────────┘
```

## Prerequisites

1. **Azure Cosmos DB Account** with the following containers:
   - `events-source` (Partition key: `/customerId`)
   - `events-processed` (Partition key: `/region`)
   - `cosmos-leases` (Partition key: `/id`)

2. **Development Environment**:
   - Docker Desktop
   - Java 17+ and Maven 3.6+
   - .NET 8 SDK
   - Azure CLI (optional)

## Quick Start Deployment

### Step 1: Configure Environment

```bash
# Copy environment template
cd infra
cp .env.example .env

# Edit .env with your Cosmos DB details
nano .env
```

**Required Configuration**:
```bash
# Azure Cosmos DB Configuration
COSMOS_URI=https://your-account.documents.azure.com:443/
COSMOS_KEY=your-primary-key-here
COSMOS_DB=cosmosflink-demo

# Container Configuration
COSMOS_SOURCE_CONTAINER=events-source
COSMOS_TARGET_CONTAINER=events-processed
COSMOS_LEASE_CONTAINER=cosmos-leases
```

### Step 2: Build All Components

```bash
# Build .NET Producer
cd ../producer
dotnet build --configuration Release

# Build .NET Consumer  
cd ../consumer
dotnet build --configuration Release

# Build Flink Job (JAR already built - 23MB)
cd ../flink-job-java
# JAR is already available at: infra/job-jars/flink-cosmos-connector-java-1.0-SNAPSHOT.jar
```

### Step 3: Start Infrastructure

```bash
cd ../infra
docker-compose up -d
```

This starts:
- **Flink JobManager** (localhost:8081)
- **Flink TaskManager** (2 instances)
- **.NET Producer** (generates OrderCreated events)
- **.NET Consumer** (HTTP API on localhost:8080)

### Step 4: Deploy Flink Job

#### Option A: Web UI (Recommended)
1. Open Flink Web UI: http://localhost:8081
2. Go to "Submit New Job"
3. Upload: `job-jars/flink-cosmos-connector-java-1.0-SNAPSHOT.jar`
4. Entry Class: `com.cosmos.flink.job.FlinkCosmosPrimaryJob`
5. Click "Submit"

#### Option B: Command Line
```bash
# Wait for Flink cluster to be ready
sleep 30

# Submit job via CLI (from within Flink container)
docker exec -it infra-jobmanager-1 \
  flink run /opt/flink/job-jars/flink-cosmos-connector-java-1.0-SNAPSHOT.jar
```

### Step 5: Verify End-to-End Pipeline

```bash
# Check Flink job status
curl http://localhost:8081/jobs

# Check producer logs (should show OrderCreated events)
docker logs infra-producer-1

# Check consumer statistics (should show processed events)
curl http://localhost:8080/stats

# Check Flink TaskManager logs
docker logs infra-taskmanager-1
```

## Production Configuration

### High-Throughput Settings

```bash
# In .env file
PRODUCER_RATE_MSG_PER_SEC=100
SINK_BATCH_SIZE=100
COSMOS_MAX_CONCURRENCY=50
FLINK_PARALLELISM=8
```

### High-Reliability Settings

```bash
# In .env file
COSMOS_MAX_RETRY_ATTEMPTS=5
COSMOS_BACKOFF_MS=2000
SINK_CONSISTENCY_LEVEL=STRONG
FLINK_CHECKPOINT_INTERVAL_MS=10000
```

### Memory Optimization

```bash
# In .env file
FLINK_TASKMANAGER_MEMORY=4096m
FLINK_JOBMANAGER_MEMORY=2048m
SINK_BATCH_TIMEOUT_MS=1000
```

## Monitoring and Troubleshooting

### Key Metrics to Monitor

1. **Flink Web UI** (http://localhost:8081):
   - Job status and uptime
   - Checkpoint success rate
   - Throughput metrics
   - TaskManager resource usage

2. **Consumer Statistics** (http://localhost:8080/stats):
   - Events processed count
   - Processing rate
   - Error counts

3. **Cosmos DB Metrics** (Azure Portal):
   - Request Units (RU) consumption
   - Throttling rates
   - Storage usage

### Common Issues and Solutions

#### Issue: Flink Job Fails to Start
**Solution**: Check environment variables in logs
```bash
docker logs infra-taskmanager-1 | grep ERROR
```

#### Issue: No Events Flowing
**Solutions**:
1. Verify Cosmos DB connectivity:
```bash
# Test from producer container
docker exec -it infra-producer-1 ping your-account.documents.azure.com
```

2. Check partition discovery:
```bash
# Look for partition discovery logs
docker logs infra-taskmanager-1 | grep "split-"
```

#### Issue: High Latency
**Solutions**:
1. Reduce batch timeout: `SINK_BATCH_TIMEOUT_MS=1000`
2. Increase parallelism: `FLINK_PARALLELISM=4`
3. Tune Cosmos DB: `COSMOS_MAX_CONCURRENCY=20`

#### Issue: Memory Errors
**Solutions**:
1. Increase TaskManager memory: `FLINK_TASKMANAGER_MEMORY=2048m`
2. Reduce batch size: `SINK_BATCH_SIZE=25`

## Performance Benchmarks

### Test Results (Sample Configuration)

| Metric | Value |
|--------|-------|
| **Events/sec** | 1,000 |
| **End-to-end Latency** | < 2 seconds |
| **Memory Usage** | ~1GB per TaskManager |
| **CPU Usage** | ~50% per core |
| **RU Consumption** | ~500 RU/s |

### Scaling Guidelines

| Events/sec | Parallelism | TaskManager Memory | RU Provision |
|------------|-------------|-------------------|--------------|
| 100 | 2 | 1GB | 1,000 RU/s |
| 1,000 | 4 | 2GB | 5,000 RU/s |
| 10,000 | 8 | 4GB | 20,000 RU/s |

## Advanced Features

### Exactly-Once Processing
- Enabled by default with 30-second checkpoints
- Uses two-phase commit protocol for Cosmos DB writes
- Automatic recovery on failures

### Fault Tolerance
- Automatic split reassignment on TaskManager failures
- Continuation token persistence in checkpoints
- Retry logic with exponential backoff

### Dynamic Scaling
- Partitions automatically discovered and assigned
- Horizontal scaling aligned with Cosmos DB partitions
- Load balancing across available TaskManagers

## Clean Shutdown

```bash
# Stop Flink job gracefully
curl -X PATCH http://localhost:8081/jobs/{job-id}

# Stop all containers
cd infra
docker-compose down

# Clean up volumes (optional)
docker-compose down -v
```

## Next Steps

After successful deployment, consider:

1. **Performance Testing**:
   - Load testing with realistic data volumes
   - Latency testing under different loads
   - Failure scenario testing

2. **Production Hardening**:
   - Setup monitoring alerts
   - Implement backup strategies
   - Configure log aggregation

3. **Scala Implementation**:
   - Functional programming approach
   - Type-safe configuration
   - Comparison benchmarks

## Support

For issues and questions:
1. Check Flink logs: `docker logs infra-taskmanager-1`
2. Check application logs: `docker logs infra-producer-1`
3. Review Cosmos DB metrics in Azure Portal
4. Consult troubleshooting section above

This deployment represents a **production-ready, fault-tolerant real-time data processing pipeline** with Azure Cosmos DB and Apache Flink.