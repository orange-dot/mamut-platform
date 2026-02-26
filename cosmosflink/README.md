# CosmosDB Flink Connectors - Universal Apache Flink Source & Sink for Azure Cosmos DB NoSQL

## Executive Summary

This project provides production-ready, highly configurable implementations of **both Source and Sink connectors** for Apache Flink with Azure Cosmos DB NoSQL. The solution demonstrates best practices for building high-performance, fault-tolerant **bi-directional data pipelines** that enable real-time streaming between Kafka/Flink and Cosmos DB while maintaining **stateful event-driven processing** capabilities and exactly-once semantics.

**Core Value Proposition**: Universal, type-agnostic connectors that seamlessly integrate Apache Flink's sophisticated state management with Azure Cosmos DB, enabling complex event processing, real-time analytics, and business process monitoring across any domain with **full bi-directional data flow**.

**Key Achievement**: Implementation of both **Custom Cosmos Source** (change feed & query-based) and **Custom Cosmos Sink** with direct, high-throughput data paths, fine-grained configurability, intelligent batching, backpressure management, and full integration with Flink's checkpointing mechanism for exactly-once processing guarantees.

## Bi-Directional Architecture: Complete Data Flow

### Why Bi-Directional Cosmos Integration Matters

Modern streaming architectures require **both ingestion to and consumption from** data stores:

- **Sink Pipeline**: Real-time event ingestion from Kafka → Flink → Cosmos DB
- **Source Pipeline**: Change feed processing and data export from Cosmos DB → Flink → Kafka
- **Unified State Management**: Consistent state handling across both data directions
- **Event-Driven Architecture**: React to database changes while simultaneously feeding the database

### Dual Implementation: Java & Scala

This project provides **feature-complete implementations in both Java and Scala**, showcasing different programming paradigms for the same functionality:

- **Java Implementation**: Enterprise-grade with familiar patterns, extensive documentation, mature tooling
- **Scala Implementation**: Functional programming approach with type safety, conciseness, and expressive syntax
- **Feature Parity**: Both implementations provide identical functionality and performance characteristics
- **Performance Comparison**: Side-by-side benchmarking capabilities

## Architecture Overview

### Complete Bi-Directional Flow

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   .NET Producer │───▶│      Apache Kafka    │───▶│  Flink Sink Job     │
│   (Docker)      │    │   (Local/Docker)     │    │  (Java/Scala)       │
└─────────────────┘    └──────────────────────┘    └─────────┬───────────┘
                                ▲                            │
                                │                            ▼
┌─────────────────┐    ┌────────┴──────────┐    ┌─────────────────────┐
│ Kafka Consumer  │◀───│ Flink Source Job  │◀───│ Azure Cosmos DB     │
│   (Optional)    │    │  (Java/Scala)     │    │   NoSQL API         │
└─────────────────┘    └───────────────────┘    │ - Change Feed       │
                                                │ - FLIP-27 Source    │
                                                │ - Multi-container   │
                                                └─────────────────────┘
```

### Custom Cosmos Source Features

#### Real-Time Change Feed Processing (FLIP-27 Architecture)
- **Modern Source API**: Built on FLIP-27 with SplitEnumerator/SourceReader separation of concerns
- **Change Feed Processor Integration**: Leverages Azure Cosmos DB CFP for fault-tolerant consumption
- **Partition-Aware**: Each Cosmos DB physical partition (`FeedRange`) becomes a `CosmosDBSplit`
- **Continuation Tokens**: Proper checkpoint management with CFP continuation tokens for exactly-once processing
- **Dynamic Load Balancing**: CFP instances coordinate via lease container for automatic partition distribution
- **Filtering**: Built-in filtering capabilities for selective change processing

#### Query-Based Batch Processing
- **Unified API**: Same FLIP-27 interface supports both bounded (query) and unbounded (change feed) modes
- **Cross-Partition Queries**: Support for SQL queries spanning multiple partitions with automatic pagination
- **Projection**: Select only required fields to minimize RU consumption
- **Time-Based Queries**: Support for time-range based data extraction with event-time watermarks

### Custom Cosmos Sink Features (Enhanced)

#### Production-Grade Write Operations
- **Upsert/Insert Modes**: Configurable write semantics
- **Bulk Operations**: Efficient batching with partition-key grouping
- **Conflict Resolution**: Configurable conflict resolution strategies
- **Idempotent Writes**: Deterministic document IDs for retry safety

## The Foundation: Stateful Stream Processing

### Why Stateful Processing Matters

Apache Flink's core strength lies in its sophisticated **stateful stream processing** capabilities, making it ideal for complex, event-driven applications where processing requires context from previous events:

- **Contextual Processing**: Maintain state across events for pattern detection and correlation
- **Real-Time Aggregations**: Running computations and windowed analytics  
- **Business Process Monitoring**: Track complex workflows and state machines
- **Temporal Operations**: Time-based computations and event-time processing

### Flink's Stateful Architecture Advantages

#### 1. Co-located Data and Computation
- **Low-Latency State Access**: State stored locally for microsecond access times
- **No Remote Lookups**: Eliminates external state query bottlenecks
- **Automatic Partitioning**: State distributed by configurable keys

#### 2. Managed State with Fault Tolerance
```java
// Example: Generic stateful processor with configurable state
public class StatefulProcessor<K, V, O> extends KeyedProcessFunction<K, V, O> {
    
    // Flink-managed state configurable per use case
    private ValueState<ProcessingState> processingState;
    private ListState<V> eventBuffer;
    private MapState<String, Object> contextState;
    
    @Override
    public void processElement(V value, Context ctx, Collector<O> out) {
        // Access state instantly (in-memory or local disk)
        ProcessingState currentState = processingState.value();
        
        // Process based on state and business logic
        O output = processWithState(value, currentState);
        if (output != null) {
            out.collect(output);
        }
        
        // Update state for future events
        updateState(value, currentState);
    }
}
```

#### 3. Checkpointing for Exactly-Once Guarantees
- **Consistent Snapshots**: Periodic snapshots of complete application state
- **Automatic Recovery**: Zero data loss recovery from checkpoints
- **Configurable Backends**: In-memory, RocksDB, or custom state backends
- **Sink Integration**: Custom Cosmos DB sink coordinates with checkpointing

## Architectural Design: Universal Sink Implementation

### Direct Integration Architecture

The sink implements a **direct pathway** (Flink → Custom Sink → Cosmos DB) optimized for:

✅ **Minimal Latency**: Direct connection without intermediaries  
✅ **Type Flexibility**: Generic implementation supporting any POJO or Avro schema  
✅ **Full Configurability**: Extensive configuration options for all aspects  
✅ **State Integration**: Seamless coordination with Flink's stateful processing  
✅ **Resource Efficiency**: Optimized RU consumption and connection management  

### Architecture Overview

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Flink Stream  │───▶│  AsyncSinkBase       │───▶│  Azure Cosmos DB    │
│                 │    │  Universal Sink      │    │  NoSQL Container    │
│ [Any Data Type] │    │  - Type Agnostic     │    │  - Partitioned      │
│ [Configurable]  │    │  - Batching          │    │  - Auto-scaling     │
│ [Stateful Ops]  │    │  - Async I/O         │    │  - RU Optimized     │
│                 │    │  - Backpressure      │    │  - Global Dist.     │
└─────────────────┘    └──────────────────────┘    └─────────────────────┘
                                │
                       ┌────────▼────────┐
                       │ Checkpoint Store │
                       │ - Configurable   │
                       │ - Exactly-once   │
                       └─────────────────┘
```

### Core Technical Components

1. **Generic AsyncSinkBase**: Type-parameterized implementation for any data model
2. **Configurable Serialization**: Pluggable serializers (JSON, Avro, Protobuf)
3. **Dynamic Configuration**: Runtime-configurable behavior without recompilation
4. **Intelligent Batching**: Configurable strategies for optimal throughput
5. **Advanced Error Handling**: Customizable retry and failure policies
6. **Comprehensive Monitoring**: Built-in metrics and diagnostics

## Implementation Features

### 1. Universal Bi-Directional Support

```java
// Generic source supporting any data type from Cosmos DB
public class CosmosDBSource<T> extends Source<T, CosmosDBSourceSplit, CosmosDBSourceState> {
    
    private final SerializationSchema<T> serializationSchema;
    private final DocumentDeserializer<T> deserializer;
    private final QueryConfiguration queryConfig;
    
    public CosmosDBSource(
            CosmosDBSourceConfig config,
            DocumentDeserializer<T> deserializer,
            QueryConfiguration queryConfig) {
        // Universal implementation for any type T from Cosmos DB
    }
}

// Generic sink supporting any data type to Cosmos DB
public class CosmosDBSink<T> extends AsyncSinkBase<T, CosmosDBRequestEntry> {
    
    private final SerializationSchema<T> serializationSchema;
    private final PartitionKeyExtractor<T> partitionKeyExtractor;
    private final DocumentIdExtractor<T> idExtractor;
    
    public CosmosDBSink(
            CosmosDBSinkConfig config,
            SerializationSchema<T> serializationSchema,
            PartitionKeyExtractor<T> partitionKeyExtractor,
            DocumentIdExtractor<T> idExtractor) {
        // Universal implementation for any type T to Cosmos DB
    }
}
```

### 2. Comprehensive Source Configuration

```java
@ConfigurationProperties(prefix = "cosmos.source")
public class CosmosDBSourceConfig {
    
    // Connection Configuration
    private String endpoint;
    private String database;
    private String container;
    private AuthenticationMode authMode;
    
    // Source Mode Configuration
    private SourceMode sourceMode = SourceMode.CHANGE_FEED;
    private String customQuery;
    private Duration pollInterval = Duration.ofSeconds(5);
    
    // Change Feed Processor Configuration
    private String changeFeedContainerName;
    private String leaseContainerName;
    private String hostName;
    private String leasePrefix;
    private Duration changeFeedStartTime;
    private int maxItemCount = 1000;
    
    // FLIP-27 Source Configuration
    private int splitDiscoveryInterval = 30; // seconds
    private int maxSplitsPerReader = 4;
    private boolean enableDynamicSplitAssignment = true;
    
    // Query Configuration
    private boolean enableCrossPartitionQuery = true;
    private Duration queryTimeout = Duration.ofMinutes(5);
    private int maxDegreeOfParallelism = 8;
    private int prefetchCount = 100;
    
    // Performance Tuning
    private ResponseContinuationTokenLimitInKb continuationTokenLimit = 1;
    private RetryPolicy retryPolicy;
    private int maxRetryAttempts = 3;
    private Duration maxRetryWaitTime = Duration.ofSeconds(30);
    
    // Monitoring
    private boolean enableMetrics = true;
    private boolean enableDiagnostics = false;
}

public enum SourceMode {
    CHANGE_FEED,        // Real-time change feed processing
    QUERY_CONTINUOUS,   // Continuous query execution
    QUERY_BATCH,        // One-time query execution
    HYBRID              // Combination of query + change feed
}
```

### 3. Advanced Source Features (FLIP-27 Implementation)

```java
// FLIP-27 Source Implementation
public class CosmosDBSource<T> implements Source<T, CosmosDBSplit, CosmosDBSourceState> {
    
    private final CosmosDBSourceConfig config;
    private final DocumentDeserializer<T> deserializer;
    private final QueryConfiguration queryConfig;
    
    @Override
    public SplitEnumerator<CosmosDBSplit, CosmosDBSourceState> createEnumerator(
            SplitEnumeratorContext<CosmosDBSplit> context) {
        return new CosmosDBSplitEnumerator(context, config);
    }
    
    @Override
    public SourceReader<T, CosmosDBSplit> createReader(
            SourceReaderContext context) {
        return new CosmosDBSourceReader<>(context, config, deserializer);
    }
    
    @Override
    public Boundedness getBoundedness() {
        return config.getSourceMode() == SourceMode.CHANGE_FEED 
            ? Boundedness.CONTINUOUS_UNBOUNDED 
            : Boundedness.BOUNDED;
    }
}

// Split Enumerator - Discovers and assigns partitions
public class CosmosDBSplitEnumerator implements SplitEnumerator<CosmosDBSplit, CosmosDBSourceState> {
    
    @Override
    public void start() {
        // Discover Cosmos DB physical partitions using FeedRange API
        List<FeedRange> feedRanges = cosmosContainer.getFeedRanges().block();
        
        List<CosmosDBSplit> splits = feedRanges.stream()
            .map(feedRange -> new CosmosDBSplit(feedRange, null))
            .collect(Collectors.toList());
            
        this.pendingSplits.addAll(splits);
        assignSplits();
    }
    
    @Override
    public void handleSplitRequest(int subtaskId, @Nullable String requesterHostname) {
        if (!pendingSplits.isEmpty()) {
            CosmosDBSplit split = pendingSplits.poll();
            context.assignSplit(split, subtaskId);
        } else {
            context.signalNoMoreSplits(subtaskId);
        }
    }
    
    @Override
    public void addSplitsBack(List<CosmosDBSplit> splits, int subtaskId) {
        // Handle splits from failed readers
        pendingSplits.addAll(splits);
        assignSplits();
    }
}

// Source Reader - Consumes data using Change Feed Processor
public class CosmosDBSourceReader<T> extends SourceReaderBase<T, CosmosDBSplit, CosmosDBSplit> {
    
    private final Map<String, ChangeFeedProcessor> activeProcessors = new HashMap<>();
    
    @Override
    public void addSplits(List<CosmosDBSplit> splits) {
        for (CosmosDBSplit split : splits) {
            startChangeFeedProcessor(split);
        }
    }
    
    private void startChangeFeedProcessor(CosmosDBSplit split) {
        ChangeFeedProcessor processor = new ChangeFeedProcessorBuilder()
            .hostName(config.getHostName())
            .feedContainer(sourceContainer)
            .leaseContainer(leaseContainer)
            .handleChanges(this::handleChanges)
            .buildChangeFeedProcessor();
            
        processor.start().subscribe();
        activeProcessors.put(split.getSplitId(), processor);
    }
    
    private Mono<Void> handleChanges(List<JsonNode> changes) {
        for (JsonNode change : changes) {
            T record = deserializer.deserialize(change);
            output.collect(record);
        }
        return Mono.empty();
    }
}

// Split representation
public class CosmosDBSplit implements SourceSplit {
    private final FeedRange feedRange;
    private final String continuationToken;
    private final String splitId;
    
    // Serialization methods for checkpointing
    public CosmosDBSplit(FeedRange feedRange, String continuationToken) {
        this.feedRange = feedRange;
        this.continuationToken = continuationToken;
        this.splitId = generateSplitId(feedRange);
    }
}
```

### 4. Scala Implementation Highlights (FLIP-27 + Functional Programming)

```scala
// Scala-idiomatic FLIP-27 source with functional composition
class CosmosDBSource[T: TypeInformation](
    config: CosmosDBSourceConfig,
    deserializer: DocumentDeserializer[T],
    queryConfig: Option[QueryConfiguration] = None
) extends Source[T, CosmosDBSplit, CosmosDBSourceState] {
  
  // Pattern matching for source mode configuration
  def createEnumerator(context: SplitEnumeratorContext[CosmosDBSplit]): SplitEnumerator[CosmosDBSplit, CosmosDBSourceState] = {
    config.sourceMode match {
      case SourceMode.ChangeFeed => 
        new CosmosDBChangeFeedEnumerator(context, config)
      case SourceMode.QueryContinuous => 
        new CosmosDBQueryEnumerator(context, config, queryConfig.get)
      case SourceMode.Hybrid => 
        new CosmosDBHybridEnumerator(context, config, queryConfig)
    }
  }
  
  // Functional error handling for reader creation
  def createReader(context: SourceReaderContext): SourceReader[T, CosmosDBSplit] = {
    Try {
      new CosmosDBSourceReader[T](context, config, deserializer)
    } match {
      case Success(reader) => reader
      case Failure(exception) => 
        throw new SourceException(s"Failed to create reader: ${exception.getMessage}")
    }
  }
  
  // Boundedness determination with pattern matching
  def getBoundedness: Boundedness = config.sourceMode match {
    case SourceMode.ChangeFeed | SourceMode.QueryContinuous => Boundedness.CONTINUOUS_UNBOUNDED
    case SourceMode.QueryBatch => Boundedness.BOUNDED
    case SourceMode.Hybrid => Boundedness.CONTINUOUS_UNBOUNDED
  }
}

// Functional Split Enumerator with immutable state
class CosmosDBSplitEnumerator(
    context: SplitEnumeratorContext[CosmosDBSplit],
    config: CosmosDBSourceConfig
) extends SplitEnumerator[CosmosDBSplit, CosmosDBSourceState] {
  
  private var pendingSplits: immutable.Queue[CosmosDBSplit] = immutable.Queue.empty
  
  override def start(): Unit = {
    // Functional composition for partition discovery
    val feedRanges = for {
      container <- getCosmosContainer(config)
      ranges <- container.getFeedRanges.asScala
    } yield ranges
    
    // Transform FeedRanges to splits using functional approach
    val splits = feedRanges.map(_.asScala.map(CosmosDBSplit.fromFeedRange))
      .getOrElse(Seq.empty)
    
    pendingSplits = pendingSplits.enqueueAll(splits)
    assignSplitsToReaders()
  }
  
  override def handleSplitRequest(subtaskId: Int, requesterHostname: String): Unit = {
    pendingSplits.dequeueOption match {
      case Some((split, remaining)) =>
        pendingSplits = remaining
        context.assignSplit(split, subtaskId)
      case None =>
        context.signalNoMoreSplits(subtaskId)
    }
  }
  
  // Functional approach to split reassignment
  override def addSplitsBack(splits: util.List[CosmosDBSplit], subtaskId: Int): Unit = {
    val splitsSeq = splits.asScala.toSeq
    pendingSplits = pendingSplits.enqueueAll(splitsSeq)
    assignSplitsToReaders()
  }
}

// Functional Source Reader with monadic composition
class CosmosDBSourceReader[T: TypeInformation](
    context: SourceReaderContext,
    config: CosmosDBSourceConfig,
    deserializer: DocumentDeserializer[T]
) extends SourceReaderBase[T, CosmosDBSplit, CosmosDBSplit](
    elementsQueue = new SingleThreadFetcherManager[T, CosmosDBSplit](
      elementsQueue => new CosmosDBSplitFetcher(elementsQueue, config, deserializer),
      config
    ),
    config
) {
  
  override def addSplits(splits: util.List[CosmosDBSplit]): Unit = {
    // Pattern matching and for-comprehension for error handling
    splits.asScala.foreach { split =>
      Try {
        startChangeFeedProcessor(split)
      } match {
        case Success(_) => 
          logger.info(s"Successfully started CFP for split ${split.splitId}")
        case Failure(exception) =>
          logger.error(s"Failed to start CFP for split ${split.splitId}", exception)
          // Re-queue split for retry
          context.sendSplitRequest()
      }
    }
  }
  
  // Functional composition for Change Feed Processor setup
  private def startChangeFeedProcessor(split: CosmosDBSplit): Unit = {
    val processorBuilder = for {
      sourceContainer <- getSourceContainer(config)
      leaseContainer <- getLeaseContainer(config)
    } yield {
      new ChangeFeedProcessorBuilder()
        .hostName(config.hostName)
        .feedContainer(sourceContainer)
        .leaseContainer(leaseContainer)
        .handleChanges(handleChanges)
        .buildChangeFeedProcessor()
    }
    
    processorBuilder match {
      case Success(processor) => 
        processor.start().subscribe()
        activeProcessors += (split.splitId -> processor)
      case Failure(exception) =>
        throw new SourceException(s"Failed to build CFP: ${exception.getMessage}")
    }
  }
  
  // Functional change handling with pattern matching
  private def handleChanges(changes: util.List[JsonNode]): Mono[Void] = {
    changes.asScala.foreach { change =>
      deserializer.deserialize(change) match {
        case Success(record) =>
          output.collect(record)
        case Failure(exception) =>
          logger.warn(s"Failed to deserialize document: ${exception.getMessage}")
          // Send to DLQ or side output
      }
    }
    Mono.empty()
  }
}

// DSL for configuration with case classes
object CosmosDBSource {
  
  case class SourceBuilder[T: TypeInformation](
      config: Option[CosmosDBSourceConfig] = None,
      deserializer: Option[DocumentDeserializer[T]] = None,
      queryConfig: Option[QueryConfiguration] = None
  ) {
    
    def withConfig(cfg: CosmosDBSourceConfig): SourceBuilder[T] = 
      copy(config = Some(cfg))
    
    def withDeserializer(deser: DocumentDeserializer[T]): SourceBuilder[T] = 
      copy(deserializer = Some(deser))
    
    def withQuery(query: QueryConfiguration): SourceBuilder[T] = 
      copy(queryConfig = Some(query))
    
    def forChangeFeed: SourceBuilder[T] = {
      val updatedConfig = config.map(_.copy(sourceMode = SourceMode.ChangeFeed))
      copy(config = updatedConfig)
    }
    
    def forQuery(sqlQuery: String): SourceBuilder[T] = {
      val query = QueryConfiguration(sqlQuery, enableCrossPartition = true)
      val updatedConfig = config.map(_.copy(sourceMode = SourceMode.QueryContinuous))
      copy(config = updatedConfig, queryConfig = Some(query))
    }
    
    def build(): CosmosDBSource[T] = {
      new CosmosDBSource[T](
        config.getOrElse(throw new IllegalArgumentException("Config required")),
        deserializer.getOrElse(new JsonDocumentDeserializer[T]()),
        queryConfig
      )
    }
  }
  
  def apply[T: TypeInformation](): SourceBuilder[T] = SourceBuilder[T]()
}

// Immutable case classes for split and state management
case class CosmosDBSplit(
    feedRange: FeedRange,
    continuationToken: Option[String],
    splitId: String
) extends SourceSplit {
  
  def withContinuationToken(token: String): CosmosDBSplit = 
    copy(continuationToken = Some(token))
}

object CosmosDBSplit {
  def fromFeedRange(feedRange: FeedRange): CosmosDBSplit = {
    val splitId = s"cosmos-split-${feedRange.hashCode()}"
    CosmosDBSplit(feedRange, None, splitId)
  }
}

case class CosmosDBSourceState(
    assignedSplits: Map[Int, Seq[CosmosDBSplit]],
    pendingSplits: Seq[CosmosDBSplit]
)
```

### 5. Bi-Directional Pipeline Examples

```java
// Java - Complete bi-directional pipeline
public class BiDirectionalCosmosFlinkPipeline {
    
    public static void main(String[] args) throws Exception {
        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.enableCheckpointing(10000);
        
        // Configure source and sink
        CosmosDBSourceConfig sourceConfig = CosmosDBSourceConfig.builder()
            .endpoint("https://account.documents.azure.com:443/")
            .database("StreamingDB")
            .container("orders")
            .sourceMode(SourceMode.CHANGE_FEED)
            .build();
            
        CosmosDBSinkConfig sinkConfig = CosmosDBSinkConfig.builder()
            .endpoint("https://account.documents.azure.com:443/")
            .database("StreamingDB")
            .container("processed-orders")
            .batchingStrategy(BatchingStrategy.ADAPTIVE)
            .build();
        
        // Pipeline 1: Kafka -> Cosmos DB (Sink)
        DataStream<OrderEvent> kafkaStream = env
            .addSource(new FlinkKafkaConsumer<>("orders-topic", new OrderEventSchema(), kafkaProps))
            .keyBy(OrderEvent::getCustomerId)
            .window(TumblingEventTimeWindows.of(Time.minutes(1)))
            .aggregate(new OrderAggregator());
        
        CosmosDBSink<OrderEvent> sink = CosmosDBSink.<OrderEvent>builder()
            .withConfig(sinkConfig)
            .withPartitionKeyExtractor(order -> order.getCustomerId())
            .withIdExtractor(order -> order.getOrderId())
            .build();
        
        kafkaStream.addSink(sink);
        
        // Pipeline 2: Cosmos DB -> Kafka (FLIP-27 Source)
        DataStream<ProcessedOrder> cosmosStream = env
            .addSource(new CosmosDBSource<>(
                sourceConfig,
                new JsonDocumentDeserializer<>(ProcessedOrder.class)
            ))
            .filter(order -> order.getStatus().equals("COMPLETED"))
            .map(new OrderEnrichmentFunction());
        
        FlinkKafkaProducer<ProcessedOrder> kafkaProducer = new FlinkKafkaProducer<>(
            "processed-orders-topic",
            new ProcessedOrderSchema(),
            kafkaProps
        );
        
        cosmosStream.addSink(kafkaProducer);
        
        env.execute("Bi-Directional Cosmos-Flink Pipeline");
    }
}
```

```scala
// Scala - Functional bi-directional pipeline
object BiDirectionalCosmosFlinkPipeline extends App {
  
  val env = StreamExecutionEnvironment.getExecutionEnvironment
  env.enableCheckpointing(10000)
  
  // Configuration
  val sourceConfig = CosmosDBSourceConfig(
    endpoint = "https://account.documents.azure.com:443/",
    database = "StreamingDB",
    container = "orders",
    sourceMode = SourceMode.ChangeFeed
  )
  
  val sinkConfig = CosmosDBSinkConfig(
    endpoint = "https://account.documents.azure.com:443/",
    database = "StreamingDB",
    container = "processed-orders"
  )
  
  // Case classes for type safety
  case class OrderEvent(orderId: String, customerId: String, amount: Double, timestamp: Long)
  case class ProcessedOrder(orderId: String, customerId: String, totalAmount: Double, status: String)
  
  // Pipeline 1: Kafka -> Cosmos DB (Functional style)
  val kafkaToCosmosFlow = env
    .addSource(new FlinkKafkaConsumer[OrderEvent]("orders-topic", new OrderEventSchema(), kafkaProps))
    .keyBy(_.customerId)
    .window(TumblingEventTimeWindows.of(Time.minutes(1)))
    .aggregate(new OrderAggregator())
    .addSink(
      CosmosDBSink[OrderEvent]()
        .withConfig(sinkConfig)
        .withPartitionKey(_.customerId)
        .withIdExtractor(_.orderId)
        .build()
    )
  
  // Pipeline 2: Cosmos DB -> Kafka (Functional style with FLIP-27)
  val cosmosToKafkaFlow = env
    .addSource(
      CosmosDBSource[ProcessedOrder]()
        .withConfig(sourceConfig)
        .forChangeFeed
        .build()
    )
    .filter(_.status == "COMPLETED")
    .map(enrichOrder)
    .addSink(new FlinkKafkaProducer[ProcessedOrder](
      "processed-orders-topic",
      new ProcessedOrderSchema(),
      kafkaProps
    ))
  
  env.execute("Bi-Directional Cosmos-Flink Pipeline")
  
  // Functional transformation
  def enrichOrder(order: ProcessedOrder): ProcessedOrder = {
    order.copy(
      totalAmount = order.totalAmount * 1.1, // Add tax
      status = "ENRICHED"
    )
  }
}
```

### 3. Pluggable Serialization

```java
// Support for different serialization formats
public interface SerializationSchema<T> extends Serializable {
    byte[] serialize(T element);
    String getSchemaVersion();
}

// JSON serialization (default)
public class JsonSerializationSchema<T> implements SerializationSchema<T> {
    private final ObjectMapper objectMapper;
    // Implementation...
}

// Avro serialization
public class AvroSerializationSchema<T> implements SerializationSchema<T> {
    private final Schema schema;
    // Implementation...
}

// Protobuf serialization
public class ProtobufSerializationSchema<T> implements SerializationSchema<T> {
    private final Parser<T> parser;
    // Implementation...
}
```

### 4. Dynamic Partition Key Strategies

```java
// Configurable partition key extraction
public interface PartitionKeyExtractor<T> extends Serializable {
    String extractPartitionKey(T element);
}

// Field-based extraction
public class FieldPartitionKeyExtractor<T> implements PartitionKeyExtractor<T> {
    private final String fieldName;
    // Reflection-based implementation
}

// Hash-based distribution
public class HashPartitionKeyExtractor<T> implements PartitionKeyExtractor<T> {
    private final int numberOfPartitions;
    private final HashFunction hashFunction;
    // Implementation...
}

// Composite key extraction
public class CompositePartitionKeyExtractor<T> implements PartitionKeyExtractor<T> {
    private final List<String> fields;
    private final String delimiter;
    // Implementation...
}

// Time-based partitioning
public class TimeBasedPartitionKeyExtractor<T> implements PartitionKeyExtractor<T> {
    private final TimeGranularity granularity;
    // Implementation...
}
```

### 5. Exactly-Once Semantics

```java
// Checkpoint-aware implementation
public class CosmosDBSinkWriter<T> extends AsyncSinkWriter<T, CosmosDBRequestEntry> {
    
    @Override
    protected void submitRequestEntries(
            List<CosmosDBRequestEntry> requestEntries,
            ResultHandler<CosmosDBRequestEntry> resultHandler) {
        
        // Group by partition key for efficiency
        Map<String, List<CosmosDBRequestEntry>> partitionedEntries = 
            groupByPartitionKey(requestEntries);
        
        // Execute bulk operations with idempotent writes
        partitionedEntries.forEach((partitionKey, entries) -> {
            executeBulkOperations(entries, resultHandler);
        });
    }
    
    // Participates in checkpointing
    @Override
    public List<CosmosDBRequestEntry> snapshotState(long checkpointId) {
        // Return in-flight requests for checkpoint
        return getPendingRequestEntries();
    }
}
```

## Data Model Examples

### Generic Document Structure

```json
{
  "id": "unique_identifier",
  "partitionKey": "configurable_partition_value",
  "timestamp": "2025-01-15T10:30:00.123Z",
  "data": {
    // Your domain-specific data
  },
  "metadata": {
    "source": "string",
    "version": "string",
    "processingTime": "2025-01-15T10:30:01.456Z",
    "checkpointId": 12345
  }
}
```

### Configuration Examples for Different Use Cases

#### IoT Sensor Data
```properties
cosmos.sink.partition-key.strategy=TIME_BASED
cosmos.sink.partition-key.time-granularity=HOURLY
cosmos.sink.serialization.format=JSON
cosmos.sink.batching.strategy=SIZE_BASED
cosmos.sink.batching.max-size=500
```

#### Financial Transactions
```properties
cosmos.sink.partition-key.strategy=FIELD_BASED
cosmos.sink.partition-key.field=accountId
cosmos.sink.write-mode=INSERT
cosmos.sink.consistency-level=STRONG
cosmos.sink.conflict-resolution=FAIL_ON_CONFLICT
```

#### Log Aggregation
```properties
cosmos.sink.partition-key.strategy=HASH_BASED
cosmos.sink.partition-key.hash-buckets=100
cosmos.sink.batching.strategy=TIME_BASED
cosmos.sink.batching.timeout=10s
cosmos.sink.compression.enabled=true
```

## Performance Optimization

### Configurable Batching Strategies

```java
public enum BatchingStrategy {
    SIZE_BASED,      // Batch by number of records
    BYTES_BASED,     // Batch by total size in bytes
    TIME_BASED,      // Batch by time window
    ADAPTIVE,        // Dynamic batching based on throughput
    PARTITION_AWARE  // Batch by partition key
}

// Adaptive batching implementation
public class AdaptiveBatchingStrategy implements BatchingStrategy {
    private int currentBatchSize;
    private final ThroughputMonitor monitor;
    
    public int getOptimalBatchSize() {
        // Adjust based on current throughput and latency
        double throughput = monitor.getCurrentThroughput();
        double latency = monitor.getAverageLatency();
        
        if (latency > TARGET_LATENCY) {
            currentBatchSize = Math.max(MIN_BATCH_SIZE, currentBatchSize - 10);
        } else if (throughput < TARGET_THROUGHPUT) {
            currentBatchSize = Math.min(MAX_BATCH_SIZE, currentBatchSize + 10);
        }
        
        return currentBatchSize;
    }
}
```

### Request Unit (RU) Optimization

```java
public class RUOptimizer {
    
    // Intelligent indexing policy generation
    public IndexingPolicy generateOptimalIndexingPolicy(
            Schema dataSchema, 
            QueryPatterns queryPatterns) {
        
        IndexingPolicy policy = new IndexingPolicy();
        
        // Include only queried fields
        queryPatterns.getQueriedFields().forEach(field -> 
            policy.addIncludedPath(new IncludedPath(field))
        );
        
        // Exclude large or rarely queried fields
        dataSchema.getLargeFields().forEach(field ->
            policy.addExcludedPath(new ExcludedPath(field))
        );
        
        return policy;
    }
    
    // Document size optimization
    public <T> T optimizeDocument(T document, OptimizationStrategy strategy) {
        switch (strategy) {
            case COMPRESSION:
                return compressLargeFields(document);
            case FIELD_REMOVAL:
                return removeNonEssentialFields(document);
            case NORMALIZATION:
                return normalizeRepeatingData(document);
            default:
                return document;
        }
    }
}
```

## Multi-Language Implementation

This project provides first-class support for Java, Scala, and Python with feature parity across all languages. Each implementation follows the idiomatic patterns and best practices of its respective language while maintaining consistent functionality.

### Java Implementation

#### Core Sink Class
```java
package com.cosmos.flink.sink;

import org.apache.flink.connector.base.sink.AsyncSinkBase;
import org.apache.flink.connector.base.sink.writer.BufferedRequestState;
import org.apache.flink.connector.base.sink.writer.ElementConverter;
import org.apache.flink.core.io.SimpleVersionedSerializer;

public class CosmosDBSink<T> extends AsyncSinkBase<T, CosmosDBRequestEntry> {
    
    private final CosmosDBSinkConfig config;
    private final SerializationSchema<T> serializationSchema;
    private final PartitionKeyExtractor<T> partitionKeyExtractor;
    private final DocumentIdExtractor<T> idExtractor;
    
    protected CosmosDBSink(
            CosmosDBSinkConfig config,
            SerializationSchema<T> serializationSchema,
            PartitionKeyExtractor<T> partitionKeyExtractor,
            DocumentIdExtractor<T> idExtractor,
            int maxBatchSize,
            int maxInFlightRequests,
            int maxBufferedRequests,
            long batchMaxTimeInMillis,
            long requestTimeoutMs) {
        
        super(
            new CosmosDBElementConverter<>(
                serializationSchema, 
                partitionKeyExtractor, 
                idExtractor
            ),
            maxBatchSize,
            maxInFlightRequests,
            maxBufferedRequests,
            batchMaxTimeInMillis,
            requestTimeoutMs
        );
        
        this.config = config;
        this.serializationSchema = serializationSchema;
        this.partitionKeyExtractor = partitionKeyExtractor;
        this.idExtractor = idExtractor;
    }
    
    @Override
    public StatefulSinkWriter<T, BufferedRequestState<CosmosDBRequestEntry>> createWriter(
            InitContext context) {
        return new CosmosDBSinkWriter<>(
            context,
            config,
            serializationSchema,
            partitionKeyExtractor,
            idExtractor
        );
    }
    
    @Override
    public SimpleVersionedSerializer<BufferedRequestState<CosmosDBRequestEntry>> getWriterStateSerializer() {
        return new CosmosDBStateSerializer();
    }
    
    // Fluent Builder
    public static <T> Builder<T> builder() {
        return new Builder<>();
    }
    
    public static class Builder<T> {
        private CosmosDBSinkConfig config;
        private SerializationSchema<T> serializationSchema;
        private PartitionKeyExtractor<T> partitionKeyExtractor;
        private DocumentIdExtractor<T> idExtractor;
        private int maxBatchSize = 100;
        private int maxInFlightRequests = 50;
        
        public Builder<T> withConfig(CosmosDBSinkConfig config) {
            this.config = config;
            return this;
        }
        
        public Builder<T> withSerializationSchema(SerializationSchema<T> schema) {
            this.serializationSchema = schema;
            return this;
        }
        
        public Builder<T> withPartitionKeyExtractor(PartitionKeyExtractor<T> extractor) {
            this.partitionKeyExtractor = extractor;
            return this;
        }
        
        public Builder<T> withIdExtractor(DocumentIdExtractor<T> extractor) {
            this.idExtractor = extractor;
            return this;
        }
        
        public CosmosDBSink<T> build() {
            Preconditions.checkNotNull(config, "Config is required");
            Preconditions.checkNotNull(serializationSchema, "SerializationSchema is required");
            Preconditions.checkNotNull(partitionKeyExtractor, "PartitionKeyExtractor is required");
            Preconditions.checkNotNull(idExtractor, "DocumentIdExtractor is required");
            
            return new CosmosDBSink<>(
                config,
                serializationSchema,
                partitionKeyExtractor,
                idExtractor,
                maxBatchSize,
                maxInFlightRequests,
                config.getMaxBufferedRequests(),
                config.getBatchTimeoutMs(),
                config.getRequestTimeoutMs()
            );
        }
    }
}
```

#### Usage Example
```java
// Java usage with full type safety and builder pattern
public class StreamingPipeline {
    public static void main(String[] args) throws Exception {
        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.enableCheckpointing(10000);
        
        // Configure the sink
        CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
            .endpoint("https://account.documents.azure.com:443/")
            .authMode(AuthenticationMode.KEY)
            .key(System.getenv("COSMOS_KEY"))
            .database("StreamingDB")
            .container("Events")
            .consistencyLevel(ConsistencyLevel.SESSION)
            .batchingStrategy(BatchingStrategy.ADAPTIVE)
            .build();
        
        // Create sink with custom extractors
        CosmosDBSink<MyEvent> sink = CosmosDBSink.<MyEvent>builder()
            .withConfig(config)
            .withSerializationSchema(new JsonSerializationSchema<>(MyEvent.class))
            .withPartitionKeyExtractor(event -> event.getCategory() + "_" + event.getRegion())
            .withIdExtractor(event -> event.getEventId())
            .build();
        
        // Build pipeline
        DataStream<MyEvent> stream = env
            .addSource(new MyEventSource())
            .keyBy(MyEvent::getCategory)
            .window(TumblingEventTimeWindows.of(Time.minutes(1)))
            .aggregate(new MyEventAggregator())
            .addSink(sink);
        
        env.execute("Cosmos DB Streaming Pipeline");
    }
}
```

### Scala Implementation

#### Core Sink Class
```scala
package com.cosmos.flink.sink.scala

import org.apache.flink.connector.base.sink.AsyncSinkBase
import org.apache.flink.streaming.api.scala._
import com.azure.cosmos.CosmosAsyncClient
import scala.concurrent.{Future, Promise}
import scala.concurrent.ExecutionContext.Implicits.global
import java.time.Duration

class CosmosDBSink[T: TypeInformation](
    config: CosmosDBSinkConfig,
    serializationSchema: SerializationSchema[T],
    partitionKeyExtractor: T => String,
    idExtractor: T => String
) extends AsyncSinkBase[T, CosmosDBRequestEntry](
    new ScalaElementConverter(serializationSchema, partitionKeyExtractor, idExtractor),
    config.maxBatchSize,
    config.maxInFlightRequests,
    config.maxBufferedRequests,
    config.batchTimeoutMs,
    config.requestTimeoutMs
) {
  
  override def createWriter(context: InitContext) = {
    new CosmosDBSinkWriter[T](
      context,
      config,
      serializationSchema,
      partitionKeyExtractor,
      idExtractor
    )
  }
  
  override def getWriterStateSerializer = new CosmosDBStateSerializer()
}

// Scala-idiomatic companion object with implicits
object CosmosDBSink {
  
  // Implicit conversions for common types
  implicit def jsonSerializer[T: TypeInformation]: SerializationSchema[T] = 
    new JsonSerializationSchema[T]()
  
  implicit def defaultPartitionKey[T]: T => String = 
    (t: T) => t.hashCode().toString
  
  implicit def defaultIdExtractor[T]: T => String = 
    (t: T) => java.util.UUID.randomUUID().toString
  
  // DSL for configuration
  case class SinkBuilder[T: TypeInformation](
      config: Option[CosmosDBSinkConfig] = None,
      serializationSchema: Option[SerializationSchema[T]] = None,
      partitionKeyExtractor: Option[T => String] = None,
      idExtractor: Option[T => String] = None
  ) {
    
    def withConfig(cfg: CosmosDBSinkConfig): SinkBuilder[T] = 
      copy(config = Some(cfg))
    
    def withSerializer(schema: SerializationSchema[T]): SinkBuilder[T] = 
      copy(serializationSchema = Some(schema))
    
    def withPartitionKey(extractor: T => String): SinkBuilder[T] = 
      copy(partitionKeyExtractor = Some(extractor))
    
    def withIdExtractor(extractor: T => String): SinkBuilder[T] = 
      copy(idExtractor = Some(extractor))
    
    def build()(implicit 
        defaultSerializer: SerializationSchema[T],
        defaultPartitionKey: T => String,
        defaultId: T => String
    ): CosmosDBSink[T] = {
      new CosmosDBSink[T](
        config.getOrElse(throw new IllegalArgumentException("Config required")),
        serializationSchema.getOrElse(defaultSerializer),
        partitionKeyExtractor.getOrElse(defaultPartitionKey),
        idExtractor.getOrElse(defaultId)
      )
    }
  }
  
  def apply[T: TypeInformation](): SinkBuilder[T] = SinkBuilder[T]()
}

// Enhanced Scala configuration with type-safe builder
case class CosmosDBSinkConfig(
    endpoint: String,
    authMode: AuthenticationMode = AuthenticationMode.Key,
    key: Option[String] = None,
    database: String,
    container: String,
    consistencyLevel: ConsistencyLevel = ConsistencyLevel.Session,
    batchingStrategy: BatchingStrategy = BatchingStrategy.Adaptive,
    maxBatchSize: Int = 100,
    maxInFlightRequests: Int = 50,
    maxBufferedRequests: Int = 10000,
    batchTimeoutMs: Long = 5000,
    requestTimeoutMs: Long = 60000,
    retryPolicy: RetryPolicy = RetryPolicy.ExponentialBackoff,
    enableMetrics: Boolean = true
) {
  def withEndpoint(ep: String): CosmosDBSinkConfig = copy(endpoint = ep)
  def withDatabase(db: String): CosmosDBSinkConfig = copy(database = db)
  def withContainer(cont: String): CosmosDBSinkConfig = copy(container = cont)
  def withKey(k: String): CosmosDBSinkConfig = copy(key = Some(k))
  def withBatchSize(size: Int): CosmosDBSinkConfig = copy(maxBatchSize = size)
}
```

#### Usage Example
```scala
// Scala usage with implicit conversions and functional style
import com.cosmos.flink.sink.scala._
import org.apache.flink.streaming.api.scala._

object StreamingPipeline extends App {
  
  val env = StreamExecutionEnvironment.getExecutionEnvironment
  env.enableCheckpointing(10000)
  
  // Scala-style configuration
  val config = CosmosDBSinkConfig(
    endpoint = "https://account.documents.azure.com:443/",
    database = "StreamingDB",
    container = "Events"
  ).withKey(sys.env("COSMOS_KEY"))
   .withBatchSize(500)
  
  // Case class for events
  case class MyEvent(
    eventId: String,
    category: String,
    region: String,
    value: Double,
    timestamp: Long
  )
  
  // Create sink with Scala DSL
  val sink = CosmosDBSink[MyEvent]()
    .withConfig(config)
    .withPartitionKey(e => s"${e.category}_${e.region}")
    .withIdExtractor(_.eventId)
    .build()
  
  // Functional pipeline composition
  val pipeline = env
    .addSource(new MyEventSource)
    .map(enrichEvent)
    .filter(_.value > 0)
    .keyBy(_.category)
    .window(TumblingEventTimeWindows.of(Time.minutes(1)))
    .reduce((a, b) => a.copy(value = a.value + b.value))
    .addSink(sink)
  
  env.execute("Cosmos DB Streaming Pipeline")
  
  def enrichEvent(event: MyEvent): MyEvent = {
    event.copy(timestamp = System.currentTimeMillis())
  }
}

// Advanced Scala features
object AdvancedFeatures {
  
  // Type classes for serialization
  trait CosmosSerializer[T] {
    def serialize(value: T): Array[Byte]
    def deserialize(bytes: Array[Byte]): T
  }
  
  object CosmosSerializer {
    implicit val stringSerializer: CosmosSerializer[String] = new CosmosSerializer[String] {
      def serialize(value: String): Array[Byte] = value.getBytes("UTF-8")
      def deserialize(bytes: Array[Byte]): String = new String(bytes, "UTF-8")
    }
    
    // Automatic derivation for case classes
    implicit def caseClassSerializer[T <: Product]: CosmosSerializer[T] = 
      macro SerializerMacros.materialize[T]
  }
  
  // Monadic error handling
  sealed trait SinkResult[+T]
  case class Success[T](value: T) extends SinkResult[T]
  case class Failure(error: Throwable) extends SinkResult[Nothing]
  
  // Async operations with Future
  class AsyncCosmosOperations {
    def writeAsync[T](items: Seq[T])(implicit serializer: CosmosSerializer[T]): Future[SinkResult[Int]] = {
      Future {
        try {
          // Perform write operations
          Success(items.length)
        } catch {
          case e: Exception => Failure(e)
        }
      }
    }
  }
}
```

### Python (PyFlink) Implementation

#### Core Sink Implementation
```python
# cosmos_db_sink.py
from pyflink.datastream.connectors import Sink, SinkWriter
from pyflink.datastream import RuntimeContext
from pyflink.common import Types, Configuration
from typing import Generic, TypeVar, Optional, Dict, Any, List, Callable
from abc import ABC, abstractmethod
import json
import asyncio
from azure.cosmos.aio import CosmosClient
from azure.cosmos import PartitionKey
from dataclasses import dataclass, field
from enum import Enum
import logging

T = TypeVar('T')

class BatchingStrategy(Enum):
    SIZE_BASED = "size_based"
    TIME_BASED = "time_based"
    ADAPTIVE = "adaptive"
    PARTITION_AWARE = "partition_aware"

class ConsistencyLevel(Enum):
    EVENTUAL = "Eventual"
    SESSION = "Session"
    BOUNDED_STALENESS = "BoundedStaleness"
    STRONG = "Strong"

@dataclass
class CosmosDBSinkConfig:
    """Configuration for Cosmos DB Sink"""
    endpoint: str
    database: str
    container: str
    auth_key: Optional[str] = None
    consistency_level: ConsistencyLevel = ConsistencyLevel.SESSION
    batching_strategy: BatchingStrategy = BatchingStrategy.ADAPTIVE
    max_batch_size: int = 100
    batch_timeout_ms: int = 5000
    max_in_flight_requests: int = 50
    enable_metrics: bool = True
    retry_policy: Dict[str, Any] = field(default_factory=lambda: {
        "max_attempts": 3,
        "backoff_multiplier": 2.0,
        "max_wait_time": 30
    })
    
    @classmethod
    def from_dict(cls, config_dict: Dict[str, Any]) -> 'CosmosDBSinkConfig':
        """Create config from dictionary"""
        return cls(**config_dict)
    
    def to_configuration(self) -> Configuration:
        """Convert to Flink Configuration"""
        config = Configuration()
        config.set_string("cosmos.endpoint", self.endpoint)
        config.set_string("cosmos.database", self.database)
        config.set_string("cosmos.container", self.container)
        config.set_integer("cosmos.batch.size", self.max_batch_size)
        return config

class SerializationSchema(ABC, Generic[T]):
    """Base serialization schema"""
    
    @abstractmethod
    def serialize(self, element: T) -> bytes:
        pass
    
    @abstractmethod
    def get_produced_type(self) -> Types:
        pass

class JsonSerializationSchema(SerializationSchema[T]):
    """JSON serialization for Python objects"""
    
    def __init__(self, type_info: Types = None):
        self.type_info = type_info or Types.PICKLED_BYTE_ARRAY()
    
    def serialize(self, element: T) -> bytes:
        if hasattr(element, '__dict__'):
            return json.dumps(element.__dict__).encode('utf-8')
        return json.dumps(element).encode('utf-8')
    
    def get_produced_type(self) -> Types:
        return self.type_info

class CosmosDBSinkWriter(SinkWriter, Generic[T]):
    """Async sink writer for Cosmos DB"""
    
    def __init__(
        self,
        config: CosmosDBSinkConfig,
        serialization_schema: SerializationSchema[T],
        partition_key_extractor: Callable[[T], str],
        id_extractor: Callable[[T], str],
        context: RuntimeContext
    ):
        self.config = config
        self.serialization_schema = serialization_schema
        self.partition_key_extractor = partition_key_extractor
        self.id_extractor = id_extractor
        self.context = context
        
        # Initialize async client
        self.client = None
        self.database = None
        self.container = None
        
        # Batching
        self.batch = []
        self.batch_size = 0
        self.last_flush_time = asyncio.get_event_loop().time()
        
        # Metrics
        self.metrics_collector = MetricsCollector() if config.enable_metrics else None
        self.logger = logging.getLogger(__name__)
    
    async def open(self):
        """Initialize Cosmos DB connection"""
        self.client = CosmosClient(
            self.config.endpoint,
            credential=self.config.auth_key,
            consistency_level=self.config.consistency_level.value
        )
        self.database = self.client.get_database_client(self.config.database)
        self.container = self.database.get_container_client(self.config.container)
    
    async def write(self, element: T, context: Any) -> None:
        """Write element to batch"""
        try:
            # Serialize element
            serialized = self.serialization_schema.serialize(element)
            document = json.loads(serialized)
            
            # Add required fields
            document['id'] = self.id_extractor(element)
            document['partitionKey'] = self.partition_key_extractor(element)
            
            # Add to batch
            self.batch.append(document)
            self.batch_size += 1
            
            # Check if should flush
            if await self._should_flush():
                await self.flush()
                
        except Exception as e:
            self.logger.error(f"Error writing element: {e}")
            if self.metrics_collector:
                self.metrics_collector.record_error()
            raise
    
    async def flush(self) -> None:
        """Flush batch to Cosmos DB"""
        if not self.batch:
            return
        
        try:
            # Group by partition key for efficiency
            partitioned_batches = self._partition_batch()
            
            # Execute bulk operations
            tasks = []
            for partition_key, documents in partitioned_batches.items():
                task = self._execute_bulk_operation(partition_key, documents)
                tasks.append(task)
            
            results = await asyncio.gather(*tasks, return_exceptions=True)
            
            # Handle results
            for result in results:
                if isinstance(result, Exception):
                    self.logger.error(f"Bulk operation failed: {result}")
                    if self.metrics_collector:
                        self.metrics_collector.record_error()
            
            # Clear batch
            self.batch.clear()
            self.batch_size = 0
            self.last_flush_time = asyncio.get_event_loop().time()
            
            if self.metrics_collector:
                self.metrics_collector.record_flush(len(results))
                
        except Exception as e:
            self.logger.error(f"Error flushing batch: {e}")
            raise
    
    async def _should_flush(self) -> bool:
        """Determine if batch should be flushed"""
        if self.config.batching_strategy == BatchingStrategy.SIZE_BASED:
            return self.batch_size >= self.config.max_batch_size
        
        elif self.config.batching_strategy == BatchingStrategy.TIME_BASED:
            current_time = asyncio.get_event_loop().time()
            return (current_time - self.last_flush_time) * 1000 >= self.config.batch_timeout_ms
        
        elif self.config.batching_strategy == BatchingStrategy.ADAPTIVE:
            # Adaptive logic based on throughput
            return self._adaptive_should_flush()
        
        return False
    
    def _adaptive_should_flush(self) -> bool:
        """Adaptive batching based on metrics"""
        if not self.metrics_collector:
            return self.batch_size >= self.config.max_batch_size
        
        throughput = self.metrics_collector.get_current_throughput()
        latency = self.metrics_collector.get_average_latency()
        
        # Adjust batch size based on performance
        if latency > 100:  # High latency, flush smaller batches
            return self.batch_size >= self.config.max_batch_size // 2
        elif throughput < 1000:  # Low throughput, accumulate larger batches
            return self.batch_size >= self.config.max_batch_size * 2
        else:
            return self.batch_size >= self.config.max_batch_size
    
    def _partition_batch(self) -> Dict[str, List[Dict]]:
        """Group documents by partition key"""
        partitioned = {}
        for doc in self.batch:
            pk = doc.get('partitionKey', 'default')
            if pk not in partitioned:
                partitioned[pk] = []
            partitioned[pk].append(doc)
        return partitioned
    
    async def _execute_bulk_operation(
        self, 
        partition_key: str, 
        documents: List[Dict]
    ) -> Dict[str, Any]:
        """Execute bulk upsert operation"""
        operations = []
        for doc in documents:
            operations.append({
                'operation': 'upsert',
                'document': doc
            })
        
        return await self.container.execute_bulk(operations)
    
    async def close(self) -> None:
        """Close resources"""
        await self.flush()
        if self.client:
            await self.client.close()

class CosmosDBSink(Sink, Generic[T]):
    """Cosmos DB Sink for PyFlink"""
    
    def __init__(
        self,
        config: CosmosDBSinkConfig,
        serialization_schema: SerializationSchema[T] = None,
        partition_key_extractor: Callable[[T], str] = None,
        id_extractor: Callable[[T], str] = None
    ):
        self.config = config
        self.serialization_schema = serialization_schema or JsonSerializationSchema()
        self.partition_key_extractor = partition_key_extractor or (lambda x: str(hash(x)))
        self.id_extractor = id_extractor or (lambda x: str(uuid.uuid4()))
    
    def create_writer(
        self, 
        context: RuntimeContext, 
        element_queue
    ) -> CosmosDBSinkWriter[T]:
        """Create sink writer instance"""
        return CosmosDBSinkWriter(
            self.config,
            self.serialization_schema,
            self.partition_key_extractor,
            self.id_extractor,
            context
        )
    
    @classmethod
    def builder(cls) -> 'CosmosDBSinkBuilder':
        """Create sink builder"""
        return CosmosDBSinkBuilder()

class CosmosDBSinkBuilder(Generic[T]):
    """Builder for Cosmos DB Sink"""
    
    def __init__(self):
        self._config = None
        self._serialization_schema = None
        self._partition_key_extractor = None
        self._id_extractor = None
    
    def with_config(self, config: CosmosDBSinkConfig) -> 'CosmosDBSinkBuilder[T]':
        self._config = config
        return self
    
    def with_serialization_schema(
        self, 
        schema: SerializationSchema[T]
    ) -> 'CosmosDBSinkBuilder[T]':
        self._serialization_schema = schema
        return self
    
    def with_partition_key_extractor(
        self, 
        extractor: Callable[[T], str]
    ) -> 'CosmosDBSinkBuilder[T]':
        self._partition_key_extractor = extractor
        return self
    
    def with_id_extractor(
        self, 
        extractor: Callable[[T], str]
    ) -> 'CosmosDBSinkBuilder[T]':
        self._id_extractor = extractor
        return self
    
    def build(self) -> CosmosDBSink[T]:
        if not self._config:
            raise ValueError("Configuration is required")
        
        return CosmosDBSink(
            self._config,
            self._serialization_schema,
            self._partition_key_extractor,
            self._id_extractor
        )

class MetricsCollector:
    """Metrics collection for monitoring"""
    
    def __init__(self):
        self.total_writes = 0
        self.total_errors = 0
        self.total_flushes = 0
        self.latencies = []
        self.throughput_samples = []
    
    def record_write(self, count: int = 1):
        self.total_writes += count
    
    def record_error(self):
        self.total_errors += 1
    
    def record_flush(self, batch_size: int):
        self.total_flushes += 1
        self.throughput_samples.append(batch_size)
    
    def record_latency(self, latency_ms: float):
        self.latencies.append(latency_ms)
    
    def get_current_throughput(self) -> float:
        if not self.throughput_samples:
            return 0
        return sum(self.throughput_samples[-10:]) / len(self.throughput_samples[-10:])
    
    def get_average_latency(self) -> float:
        if not self.latencies:
            return 0
        return sum(self.latencies[-100:]) / len(self.latencies[-100:])
```

#### Usage Example
```python
# streaming_pipeline.py
from pyflink.datastream import StreamExecutionEnvironment
from pyflink.table import StreamTableEnvironment
from pyflink.common import Types, Time
from cosmos_db_sink import (
    CosmosDBSink, 
    CosmosDBSinkConfig, 
    ConsistencyLevel,
    BatchingStrategy,
    JsonSerializationSchema
)
import os
from dataclasses import dataclass
from datetime import datetime

@dataclass
class MyEvent:
    event_id: str
    category: str
    region: str
    value: float
    timestamp: datetime

def main():
    # DataStream API Example
    env = StreamExecutionEnvironment.get_execution_environment()
    env.enable_checkpointing(10000)  # 10 seconds
    
    # Configure Cosmos DB sink
    config = CosmosDBSinkConfig(
        endpoint="https://account.documents.azure.com:443/",
        database="StreamingDB",
        container="Events",
        auth_key=os.environ.get("COSMOS_KEY"),
        consistency_level=ConsistencyLevel.SESSION,
        batching_strategy=BatchingStrategy.ADAPTIVE,
        max_batch_size=500,
        batch_timeout_ms=5000,
        max_in_flight_requests=50,
        enable_metrics=True
    )
    
    # Create sink with custom extractors
    cosmos_sink = (CosmosDBSink.builder()
        .with_config(config)
        .with_serialization_schema(JsonSerializationSchema(Types.PICKLED_BYTE_ARRAY()))
        .with_partition_key_extractor(lambda e: f"{e.category}_{e.region}")
        .with_id_extractor(lambda e: e.event_id)
        .build()
    )
    
    # Build streaming pipeline
    ds = env.from_collection([
        MyEvent("1", "cat1", "us-west", 100.0, datetime.now()),
        MyEvent("2", "cat2", "us-east", 200.0, datetime.now())
    ])
    
    ds.map(lambda x: x, output_type=Types.PICKLED_BYTE_ARRAY()) \
      .add_sink(cosmos_sink)
    
    env.execute("Cosmos DB Streaming Pipeline")

def table_api_example():
    """Example using Table API with SQL"""
    
    # Create table environment
    env_settings = EnvironmentSettings.in_streaming_mode()
    t_env = StreamTableEnvironment.create(env_settings)
    
    # Configure checkpoint
    t_env.get_config().set("execution.checkpointing.interval", "10s")
    
    # Register Cosmos DB sink table
    t_env.execute_sql("""
        CREATE TABLE cosmos_sink (
            event_id STRING,
            category STRING,
            region STRING,
            value DOUBLE,
            event_time TIMESTAMP(3),
            partition_key AS CONCAT(category, '_', region),
            PRIMARY KEY (event_id) NOT ENFORCED
        ) WITH (
            'connector' = 'cosmos-nosql',
            'cosmos.endpoint' = 'https://account.documents.azure.com:443/',
            'cosmos.database' = 'StreamingDB',
            'cosmos.container' = 'Events',
            'cosmos.auth.key' = '${sys:COSMOS_KEY}',
            'cosmos.consistency-level' = 'Session',
            'cosmos.batching.strategy' = 'ADAPTIVE',
            'cosmos.batching.max-size' = '500',
            'cosmos.batching.timeout' = '5s'
        )
    """)
    
    # Process and write to Cosmos DB
    t_env.execute_sql("""
        INSERT INTO cosmos_sink
        SELECT 
            event_id,
            category,
            region,
            SUM(value) as value,
            TUMBLE_END(event_time, INTERVAL '1' MINUTE) as event_time
        FROM source_table
        GROUP BY 
            event_id,
            category,
            region,
            TUMBLE(event_time, INTERVAL '1' MINUTE)
    """)

def advanced_features_example():
    """Advanced features and patterns"""
    
    from pyflink.datastream.window import TumblingEventTimeWindows
    from pyflink.datastream.functions import ProcessFunction, KeyedProcessFunction
    from pyflink.datastream.state import ValueStateDescriptor
    
    class StatefulProcessor(KeyedProcessFunction):
        """Stateful processing with Cosmos DB sink"""
        
        def __init__(self):
            self.state = None
        
        def open(self, runtime_context: RuntimeContext):
            descriptor = ValueStateDescriptor(
                "event_state",
                Types.PICKLED_BYTE_ARRAY()
            )
            self.state = runtime_context.get_state(descriptor)
        
        def process_element(self, value, ctx: 'KeyedProcessFunction.Context'):
            current_state = self.state.value()
            
            # Process with state
            if current_state:
                value.value += current_state.value
            
            self.state.update(value)
            
            # Output enriched event
            yield value
    
    # Use with Cosmos DB sink
    env = StreamExecutionEnvironment.get_execution_environment()
    
    config = CosmosDBSinkConfig(
        endpoint="https://account.documents.azure.com:443/",
        database="StreamingDB",
        container="ProcessedEvents",
        auth_key=os.environ.get("COSMOS_KEY")
    )
    
    sink = CosmosDBSink.builder() \
        .with_config(config) \
        .build()
    
    # Stateful pipeline
    (env.add_source(source)
        .key_by(lambda x: x.category)
        .process(StatefulProcessor())
        .add_sink(sink))
    
    env.execute("Stateful Cosmos DB Pipeline")

if __name__ == "__main__":
    main()
```

## Project Structure

```
src/
├── main/
│   ├── java/                                    # Java implementation
│   │   └── com/cosmos/flink/
│   │       ├── sink/
│   │       │   ├── CosmosDBSink.java
│   │       │   ├── CosmosDBSinkWriter.java
│   │       │   ├── CosmosDBRequestEntry.java
│   │       │   └── CosmosDBSinkBuilder.java
│   │       └── ... (other packages)
│   ├── scala/                                   # Scala implementation
│   │   └── com/cosmos/flink/sink/scala/
│   │       ├── CosmosDBSink.scala
│   │       ├── CosmosDBSinkConfig.scala
│   │       ├── Implicits.scala
│   │       └── TypeClasses.scala
│   ├── python/                                  # Python implementation
│   │   └── cosmos_flink/
│   │       ├── __init__.py
│   │       ├── cosmos_db_sink.py
│   │       ├── serialization.py
│   │       ├── config.py
│   │       ├── metrics.py
│   │       └── table_api.py
│   └── resources/
│       ├── META-INF/services/                   # Service discovery
│       ├── reference.conf                       # Default config
│       └── log4j2.xml                          # Logging config
└── test/
    ├── java/                                    # Java tests
    ├── scala/                                   # Scala tests
    └── python/                                  # Python tests
```

## Language-Specific Features Comparison

| Feature | Java | Scala | Python |
|---------|------|-------|--------|
| **Type Safety** | Strong static typing | Strong with inference | Dynamic with type hints |
| **Builder Pattern** | Fluent builder | Case class + DSL | Builder class |
| **Async Support** | CompletableFuture | Future/Promise | asyncio |
| **Serialization** | Jackson, Avro | Circe, Avro4s | json, pickle |
| **Configuration** | POJO + Builder | Case classes | Dataclasses |
| **Error Handling** | Try-catch | Try/Either | try-except |
| **Metrics** | Micrometer | Kamon | Custom collector |
| **Testing** | JUnit 5 | ScalaTest | pytest |
| **State Management** | ValueState | State monads | State descriptors |
| **Table API** | Native support | Native support | Native support |

## Quick Start

### 1. Add Dependency

```xml
<dependency>
    <groupId>com.cosmos.flink</groupId>
    <artifactId>cosmos-flink-sink</artifactId>
    <version>1.0.0</version>
</dependency>
```

### 2. Basic Usage

```java
// Define your data model
public class MyEvent {
    private String id;
    private String data;
    private long timestamp;
    // getters/setters...
}

// Create sink with minimal configuration
CosmosDBSink<MyEvent> sink = CosmosDBSink.<MyEvent>builder()
    .withConnectionString(System.getenv("COSMOS_CONNECTION_STRING"))
    .withDatabase("MyDatabase")
    .withContainer("MyContainer")
    .withSerializationSchema(new JsonSerializationSchema<>())
    .withPartitionKeyExtractor(event -> event.getPartitionKey())
    .withIdExtractor(event -> event.getId())
    .build();

// Add to your Flink pipeline
DataStream<MyEvent> stream = env.addSource(new MyEventSource());
stream.addSink(sink);
```

### 3. Advanced Configuration

```java
// Full configuration with all options
CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
    .endpoint(endpoint)
    .authMode(AuthenticationMode.MANAGED_IDENTITY)
    .database("MyDatabase")
    .container("MyContainer")
    .writeMode(WriteMode.UPSERT)
    .consistencyLevel(ConsistencyLevel.SESSION)
    .batchingStrategy(BatchingStrategy.ADAPTIVE)
    .maxBatchSize(500)
    .batchTimeout(Duration.ofSeconds(5))
    .maxInFlightRequests(50)
    .retryPolicy(RetryPolicy.EXPONENTIAL_BACKOFF)
    .maxRetryAttempts(3)
    .enableMetrics(true)
    .metricsGranularity(MetricsGranularity.SUBTASK)
    .build();

CosmosDBSink<MyEvent> sink = new CosmosDBSink<>(
    config,
    new AvroSerializationSchema<>(MyEvent.class),
    new CompositePartitionKeyExtractor<>(Arrays.asList("region", "category")),
    event -> event.getId()
);
```

## Troubleshooting

### Common Issues

#### High RU Consumption
- Review partition key cardinality
- Optimize indexing policy
- Enable compression for large documents
- Use batching strategies effectively

#### Performance Issues
- Adjust batch sizes based on document size
- Tune max-in-flight-requests
- Monitor for hot partitions
- Scale Flink parallelism appropriately

#### Checkpoint Failures
- Configure appropriate state backend
- Adjust checkpoint intervals
- Monitor state size growth
- Implement state TTL where applicable

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
  - [ ] Graceful degradation during service outages
  - [ ] State consistency verification during recovery

- [ ] **Stateful Monitoring & Observability**
  - [ ] State size and checkpoint duration metrics
  - [ ] Recovery time and state restoration monitoring
  - [ ] Watermark lag and event-time delay tracking
  - [ ] Memory usage and garbage collection optimization

### Phase 4: Multi-Language Support & Testing (Weeks 10-12)
- [ ] **PyFlink Integration**
  - [ ] DynamicTableSinkFactory implementation for Table API
  - [ ] Service provider registration
  - [ ] Python integration examples and documentation

- [ ] **Scala Support**
  - [ ] Scala wrapper classes and implicit conversions
  - [ ] Idiomatic Scala API design
  - [ ] Type-safe configuration builders

  - [ ] **Comprehensive Testing**
  - [ ] Unit tests with high coverage (>90%)
  - [ ] Integration tests with Testcontainers
  - [ ] Performance benchmarks and load testing
  - [ ] End-to-end pipeline testing

### Phase 5: Documentation & Deployment (Weeks 13-14)
- [ ] **Production Deployment**
  - [ ] Docker containerization
  - [ ] Kubernetes deployment manifests
  - [ ] Helm charts for easy deployment
  - [ ] Production configuration templates

- [ ] **Documentation & Examples**
  - [ ] Comprehensive API documentation
  - [ ] Performance tuning guides
  - [ ] Troubleshooting playbooks
  - [ ] Real-world usage examples

## Performance Benchmarks & Tuning Guide

### Expected Performance Characteristics

| Metric | Target Value | Configuration | State Considerations |
|--------|-------------|---------------|---------------------|
| **Throughput** | 10,000+ events/sec | Parallelism: 4, Batch Size: 100 | With checkpoint overhead |
| **Latency (P99)** | < 100ms end-to-end | Max In-Flight: 50, Batch Timeout: 5s | Including state access |
| **RU Efficiency** | < 10 RU per write | Optimized indexing, batch operations | State-aware batching |
| **Availability** | 99.9% uptime | Circuit breaker, retry logic | Checkpoint recovery < 30s |
| **Recovery Time** | < 30 seconds | Checkpoint interval: 10s | State restoration included |
| **Checkpoint Duration** | < 5 seconds | State backend: RocksDB | For stateful workloads |
| **State Growth** | Linear with keys | Efficient state cleanup | TTL-based state management |

### Tuning Parameters by Workload with State Considerations

#### High-Throughput Stateful Processing
```properties
# Optimize for maximum throughput with state management
sink.buffer-flush.max-rows=500
sink.buffer-flush.max-size-in-bytes=10MB
sink.buffer-flush.interval=10000
sink.max-in-flight-requests=100
cosmos.maxRetryAttemptsOnThrottledRequests=5

# Checkpoint configuration for high throughput
execution.checkpointing.interval=30000
execution.checkpointing.mode=EXACTLY_ONCE
state.backend=rocksdb
state.backend.rocksdb.predefined-options=SPINNING_DISK_OPTIMIZED
```

#### Low-Latency Real-Time Stateful Processing
```properties
# Optimize for minimal latency with frequent checkpoints
sink.buffer-flush.max-rows=50
sink.buffer-flush.max-size-in-bytes=1MB
sink.buffer-flush.interval=1000
sink.max-in-flight-requests=20
cosmos.maxRetryAttemptsOnThrottledRequests=2

# Fast checkpoint configuration
execution.checkpointing.interval=5000
execution.checkpointing.mode=EXACTLY_ONCE
state.backend=hashmap
state.backend.async=true
```

#### Large State Management
```properties
# Optimize for large state scenarios
sink.buffer-flush.max-rows=1000
sink.buffer-flush.max-size-in-bytes=16MB
sink.buffer-flush.interval=30000
cosmos.consistencyLevel=Session
cosmos.indexingPolicy=selective

# RocksDB configuration for large state
state.backend=rocksdb
state.backend.rocksdb.block.cache-size=256MB
state.backend.rocksdb.write-buffer-size=64MB
state.backend.rocksdb.compaction.level.max-size-level-base=256MB
state.checkpoints.dir=s3://your-bucket/checkpoints
```

## Quick Start Guide

### Prerequisites
- Java 11 or higher
- Apache Flink 1.17+
- Azure Cosmos DB account with NoSQL API
- Maven 3.6+ or Gradle 7+

### 1. Dependency Setup

#### Maven
```xml
<dependencies>
    <dependency>
        <groupId>org.apache.flink</groupId>
        <artifactId>flink-connector-base</artifactId>
        <version>1.17.1</version>
    </dependency>
    <dependency>
        <groupId>com.azure</groupId>
        <artifactId>azure-cosmos</artifactId>
        <version>4.50.0</version>
    </dependency>
    <dependency>
        <groupId>org.apache.flink</groupId>
        <artifactId>flink-streaming-java</artifactId>
        <version>1.17.1</version>
    </dependency>
</dependencies>
```

#### Gradle
```gradle
dependencies {
    implementation 'org.apache.flink:flink-connector-base:1.17.1'
    implementation 'com.azure:azure-cosmos:4.50.0'
    implementation 'org.apache.flink:flink-streaming-java:1.17.1'
}
```

### 2. Basic Configuration

```java
// Create sink configuration
CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
    .endpoint("https://your-cosmos-account.documents.azure.com:443/")
    .key("your-primary-key")
    .database("TaxiDispatchDB")
    .container("Events")
    .maxBatchSize(100)
    .batchTimeout(Duration.ofSeconds(5))
    .maxInFlightRequests(50)
    .consistencyLevel(ConsistencyLevel.SESSION)
    .build();
```

### 3. Simple Usage Example

```java
import com.aitaxidispatcher.sink.CosmosDBSink;
import com.aitaxidispatcher.model.TaxiEvent;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.datastream.DataStream;

public class TaxiDispatchPipeline {
    public static void main(String[] args) throws Exception {
        // Set up Flink environment
        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.enableCheckpointing(10000); // Checkpoint every 10 seconds
        
        // Create data stream (replace with your actual source)
        DataStream<TaxiEvent> taxiEventStream = env
            .addSource(new TaxiEventKafkaSource())
            .filter(event -> event.getEventType() != null)
            .map(new EventEnrichmentFunction());
        
        // Add Cosmos DB sink
        taxiEventStream.addSink(new CosmosDBSink<>(config));
        
        // Execute pipeline
        env.execute("AI Taxi Dispatcher Pipeline");
    }
}
```

### 4. PyFlink Integration Example

```python
# Table API with SQL DDL
t_env.execute_sql("""
    CREATE TABLE cosmos_sink (
        id STRING,
        data ROW<field1 STRING, field2 INT, field3 DOUBLE>,
        timestamp TIMESTAMP(3),
        partitionKey AS CAST(FLOOR(HOUR(timestamp)) AS STRING)
    ) WITH (
        'connector' = 'cosmos-nosql',
        'cosmos.endpoint' = '...',
        'cosmos.database' = 'MyDatabase',
        'cosmos.container' = 'MyContainer',
        'cosmos.write-mode' = 'UPSERT',
        'cosmos.batching.strategy' = 'ADAPTIVE',
        'cosmos.batching.max-size' = '500'
    )
""")
```

### 5. Advanced Features Example

```python
from pyflink.datastream.window import TumblingEventTimeWindows
from pyflink.datastream.functions import ProcessFunction, KeyedProcessFunction
from pyflink.datastream.state import ValueStateDescriptor

class StatefulProcessor(KeyedProcessFunction):
    """Stateful processing with Cosmos DB sink"""
    
    def __init__(self):
        self.state = None
    
    def open(self, runtime_context: RuntimeContext):
        descriptor = ValueStateDescriptor(
            "event_state",
            Types.PICKLED_BYTE_ARRAY()
        )
        self.state = runtime_context.get_state(descriptor)
    
    def process_element(self, value, ctx: 'KeyedProcessFunction.Context'):
        current_state = self.state.value()
        
        # Process with state
        if current_state:
            value.value += current_state.value
        
        self.state.update(value)
        
        # Output enriched event
        yield value

# Use with Cosmos DB sink
env = StreamExecutionEnvironment.get_execution_environment()

config = CosmosDBSinkConfig(
    endpoint="https://account.documents.azure.com:443/",
    database="StreamingDB",
    container="ProcessedEvents",
    auth_key=os.environ.get("COSMOS_KEY")
)

sink = CosmosDBSink.builder() \
    .with_config(config) \
    .build()

# Stateful pipeline
(env.add_source(source)
    .key_by(lambda x: x.category)
    .process(StatefulProcessor())
    .add_sink(sink))

env.execute("Stateful Cosmos DB Pipeline")
```

## Project Structure

```
src/
├── main/
│   ├── java/                                    # Java implementation
│   │   └── com/cosmos/flink/
│   │       ├── sink/
│   │       │   ├── CosmosDBSink.java
│   │       │   ├── CosmosDBSinkWriter.java
│   │       │   ├── CosmosDBRequestEntry.java
│   │       │   └── CosmosDBSinkBuilder.java
│   │       └── ... (other packages)
│   ├── scala/                                   # Scala implementation
│   │   └── com/cosmos/flink/sink/scala/
│   │       ├── CosmosDBSink.scala
│   │       ├── CosmosDBSinkConfig.scala
│   │       ├── Implicits.scala
│   │       └── TypeClasses.scala
│   ├── python/                                  # Python implementation
│   │   └── cosmos_flink/
│   │       ├── __init__.py
│   │       ├── cosmos_db_sink.py
│   │       ├── serialization.py
│   │       ├── config.py
│   │       ├── metrics.py
│   │       └── table_api.py
│   └── resources/
│       ├── META-INF/services/                   # Service discovery
│       ├── reference.conf                       # Default config
│       └── log4j2.xml                          # Logging config
└── test/
    ├── java/                                    # Java tests
    ├── scala/                                   # Scala tests
    └── python/                                  # Python tests
```

## Language-Specific Features Comparison

| Feature | Java | Scala | Python |
|---------|------|-------|--------|
| **Type Safety** | Strong static typing | Strong with inference | Dynamic with type hints |
| **Builder Pattern** | Fluent builder | Case class + DSL | Builder class |
| **Async Support** | CompletableFuture | Future/Promise | asyncio |
| **Serialization** | Jackson, Avro | Circe, Avro4s | json, pickle |
| **Configuration** | POJO + Builder | Case classes | Dataclasses |
| **Error Handling** | Try-catch | Try/Either | try-except |
| **Metrics** | Micrometer | Kamon | Custom collector |
| **Testing** | JUnit 5 | ScalaTest | pytest |
| **State Management** | ValueState | State monads | State descriptors |
| **Table API** | Native support | Native support | Native support |

## Quick Start

### 1. Add Dependency

```xml
<dependency>
    <groupId>com.cosmos.flink</groupId>
    <artifactId>cosmos-flink-sink</artifactId>
    <version>1.0.0</version>
</dependency>
```

### 2. Basic Usage

```java
// Define your data model
public class MyEvent {
    private String id;
    private String data;
    private long timestamp;
    // getters/setters...
}

// Create sink with minimal configuration
CosmosDBSink<MyEvent> sink = CosmosDBSink.<MyEvent>builder()
    .withConnectionString(System.getenv("COSMOS_CONNECTION_STRING"))
    .withDatabase("MyDatabase")
    .withContainer("MyContainer")
    .withSerializationSchema(new JsonSerializationSchema<>())
    .withPartitionKeyExtractor(event -> event.getPartitionKey())
    .withIdExtractor(event -> event.getId())
    .build();

// Add to your Flink pipeline
DataStream<MyEvent> stream = env.addSource(new MyEventSource());
stream.addSink(sink);
```

### 3. Advanced Configuration

```java
// Full configuration with all options
CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
    .endpoint(endpoint)
    .authMode(AuthenticationMode.MANAGED_IDENTITY)
    .database("MyDatabase")
    .container("MyContainer")
    .writeMode(WriteMode.UPSERT)
    .consistencyLevel(ConsistencyLevel.SESSION)
    .batchingStrategy(BatchingStrategy.ADAPTIVE)
    .maxBatchSize(500)
    .batchTimeout(Duration.ofSeconds(5))
    .maxInFlightRequests(50)
    .retryPolicy(RetryPolicy.EXPONENTIAL_BACKOFF)
    .maxRetryAttempts(3)
    .enableMetrics(true)
    .metricsGranularity(MetricsGranularity.SUBTASK)
    .build();

CosmosDBSink<MyEvent> sink = new CosmosDBSink<>(
    config,
    new AvroSerializationSchema<>(MyEvent.class),
    new CompositePartitionKeyExtractor<>(Arrays.asList("region", "category")),
    event -> event.getId()
);
```

## Troubleshooting

### Common Issues

#### High RU Consumption
- Review partition key cardinality
- Optimize indexing policy
- Enable compression for large documents
- Use batching strategies effectively

#### Performance Issues
- Adjust batch sizes based on document size
- Tune max-in-flight-requests
- Monitor for hot partitions
- Scale Flink parallelism appropriately

#### Checkpoint Failures
- Configure appropriate state backend
- Adjust checkpoint intervals
- Monitor state size growth
- Implement state TTL where applicable

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Apache Flink community for the robust streaming framework
- Microsoft Azure team for the excellent Cosmos DB Java SDK
- Contributors to the AsyncSinkBase API design
- Open source community for continuous feedback and improvements
