

# **Architecting a Production-Grade Apache Flink Source for Azure Cosmos DB**

## **Section 1: Introduction and Architectural Foundations**

### **1.1 The Case for a Custom Connector: Bridging Flink and Cosmos DB**

In the landscape of modern data architecture, the fusion of powerful, stateful stream processing with globally distributed, low-latency databases represents a significant strategic advantage. Apache Flink stands as a premier open-source framework for stateful computations over data streams, offering robust guarantees for exactly-once state consistency, sophisticated event-time processing, and a runtime capable of high throughput and low latency. On the other side of this architectural equation, Azure Cosmos DB provides a multi-model, globally distributed database service designed for mission-critical applications that demand predictable performance and high availability.

The strategic value of combining these two technologies is immense. It enables the development of real-time applications—from event-driven microservices to complex analytical pipelines—that can react to data as it is generated, anywhere in the world, while maintaining a consistent, stateful view of the world. However, a critical gap exists in the ecosystem: the absence of an official, dedicated source connector for integrating Flink with the Azure Cosmos DB NoSQL API. While the Flink community provides a rich set of connectors for popular systems like Apache Kafka, Amazon Kinesis, MongoDB, and Apache Cassandra, a direct, optimized bridge to Cosmos DB's NoSQL API has been a missing piece. The existence of a Microsoft-provided Kafka Connect source for Cosmos DB underscores the recognized value of such integrations, but it introduces an intermediary system (Kafka) that may not be desirable in all architectures.1

This report fills that gap by providing a definitive architectural blueprint for building a custom, production-grade Apache Flink source connector for Azure Cosmos DB. The primary integration mechanism for this connector is the Azure Cosmos DB Change Feed, a persistent, ordered log of changes to a container.2 The Change Feed is an ideal primitive for a streaming source, as it externalizes the database's event log in a consumable, fault-tolerant manner, allowing Flink to tap directly into the stream of data modifications.

### **1.2 Conceptual Overview: A High-Level View of the Data Pipeline**

The data pipeline enabled by this custom connector is conceptually straightforward yet powerful. It facilitates a direct, real-time flow of data from the persistence layer into the stream processing layer.

The process begins within Azure Cosmos DB. All insert and update operations performed on a designated "monitored container" are captured by the Change Feed feature. The Change Feed presents these changes as an ordered, persistent log that can be read by external clients.

The custom Flink source connector acts as this client. It connects to the Change Feed and ingests the stream of document changes. Once ingested, these changes are materialized as a DataStream\<T\> within the Flink application. This DataStream can then be subjected to the full suite of Flink's capabilities: stateless transformations, stateful aggregations, windowing operations, joins with other streams, and eventually, sinking the results to an external system.

At each stage, the responsibilities are clearly delineated. Cosmos DB is responsible for durable data persistence and the reliable logging of all changes. The Flink connector is responsible for the fault-tolerant ingestion of this change log. The Flink runtime, in turn, is responsible for the stateful computation and exactly-once processing of the ingested data. This separation of concerns creates a robust and scalable architecture for real-time data processing.

### **1.3 Core Technologies at Play**

A successful implementation requires a deep understanding of the fundamental models of both Flink and Cosmos DB. The architectural patterns of these two systems are not just compatible; they are conceptually homologous, a synergy that is key to a robust and elegant integration.

#### **1.3.1 Apache Flink's Stateful Stream Processing Model**

Apache Flink is a distributed processing engine architected for stateful computations over both bounded and unbounded data streams. Its core value proposition lies in its ability to manage application state with strong consistency guarantees, even in the face of failures. Key features that make Flink the ideal choice for this integration include:

* **Exactly-Once State Consistency:** Flink's checkpointing mechanism allows it to periodically snapshot the state of an entire application, ensuring that in the event of a failure, the system can restore to a consistent point without data loss or duplication.  
* **Event-Time Processing:** Flink has first-class support for processing events based on the time they occurred (event time), rather than the time they are processed. This is crucial for applications that need to handle out-of-order data and produce correct, deterministic results.  
* **High Throughput and Low Latency:** The Flink runtime is designed for performance, capable of processing millions of events per second with millisecond latencies.

#### **1.3.2 Azure Cosmos DB's Change Feed**

The Azure Cosmos DB Change Feed is a persistent log of document changes within a container, ordered by their modification time.2 It is enabled by default on all Cosmos DB accounts and captures all insert and update operations. An advanced "all versions and deletes" mode can also capture deletes, though it requires continuous backups to be enabled.

Cosmos DB provides two primary models for consuming the Change Feed 3:

1. **The Pull Model:** A low-level approach where the client is responsible for explicitly requesting changes, managing its own state (progress bookmarks), and handling load balancing across partitions. This model offers fine-grained control but comes with significant operational complexity.3  
2. **The Push Model (via Change Feed Processor):** The highly recommended approach, which utilizes the Change Feed Processor (CFP) library provided by the Cosmos DB SDK. The CFP is a sophisticated client-side library that dramatically simplifies Change Feed consumption. It provides automatic checkpointing, "at least once" delivery semantics, and dynamic load balancing of partitions across multiple consumer instances.4

For building a robust Flink source, the Change Feed Processor is the unequivocally correct choice. It abstracts away the most complex aspects of distributed stream consumption, allowing the Flink connector to focus on its primary role: bridging the Flink runtime with the Cosmos DB ecosystem. The CFP's fault-tolerant design and state management capabilities align perfectly with Flink's own architectural principles.

This alignment is not a coincidence. The architectural patterns of the CFP and Flink's modern source API are deeply synergistic. The CFP's design, which includes a "monitored container" for data, a "lease container" for state storage and coordination, and multiple "compute instances" (workers) that process partitions in parallel, is a direct parallel to Flink's own distributed architecture.4 Flink's modern source model, with its central coordinator and parallel workers, maps cleanly onto the concepts provided by the CFP. This means the Flink source connector does not need to reinvent a complex consumption pattern; instead, it acts as an intelligent adapter, translating Flink's architectural primitives into the language of the Change Feed Processor. This insight is the cornerstone of the entire connector design presented in this report.

## **Section 2: A Tale of Two APIs: Flink's Source Interfaces**

To build a connector, one must first choose the appropriate API. Apache Flink provides two distinct interfaces for creating custom sources: the legacy SourceFunction and the modern Source API, introduced in Flink Improvement Proposal 27 (FLIP-27). A thorough understanding of their differences is essential for making an informed architectural decision. The modern Source API is the definitive choice for any new connector development, a conclusion supported by its superior design, feature set, and the Flink community's strategic direction.

### **2.1 The Legacy SourceFunction: A Retrospective Analysis**

The SourceFunction interface was the original mechanism for creating custom data sources in Flink. It is a relatively simple interface, often extended via RichSourceFunction to gain access to lifecycle methods (open(), close()) and the runtime context.

The core of a SourceFunction is its run(SourceContext\<T\> ctx) method. This method contains the entire logic for the source: connecting to the external system, fetching data in a loop, emitting records to the Flink pipeline via ctx.collect(), and handling cancellation. While straightforward for simple use cases, this monolithic design presents several significant limitations in the context of a sophisticated, distributed source:

* **Lack of Separation of Concerns:** The run() method conflates the logic for discovering what work needs to be done (e.g., which partitions to read) with the logic for actually performing the work (reading data from those partitions). This makes it difficult to implement advanced coordination strategies.  
* **Difficult Unification of Batch and Streaming:** The API was designed primarily for unbounded streaming. Adapting it for bounded (batch) execution is often awkward and requires manual implementation of termination logic.  
* **Implicit Parallelism:** While Flink can run multiple parallel instances of a SourceFunction, the distribution of work among these instances is not an explicit concept in the API. Developers must implement their own coordination logic, often using external systems like Zookeeper, to ensure each parallel instance processes a unique subset of the data.  
* **Manual Watermark Emission:** For event-time processing, the run() method is responsible for manually emitting watermarks via ctx.emitWatermark(), intertwining the data emission logic with the time-tracking logic.

### **2.2 The Modern Source API (FLIP-27): A New Paradigm for Unification**

Recognizing the limitations of SourceFunction, the Flink community introduced a completely redesigned Source API as part of FLIP-27. This new API is built on a foundation of clear separation of concerns and is designed from the ground up to provide a unified interface for both batch and streaming data sources.

The FLIP-27 Source API employs a master-slave architecture, decoupling the coordination logic from the data-fetching logic:

* **SplitEnumerator:** This component acts as the coordinator or "master." It runs as a single, non-parallel instance on the Flink JobManager. Its primary responsibilities are work discovery and work assignment. It discovers the partitions of the external data source (called "splits") and assigns them to the available SourceReader instances. It also plays a crucial role in fault tolerance, reassigning splits from failed readers to healthy ones.  
* **SourceReader:** This component is the parallel worker. Multiple instances of the SourceReader run on the Flink TaskManagers. Each reader requests splits from the SplitEnumerator, reads the data corresponding to its assigned splits, and emits the records and watermarks into the Flink data stream. Flink provides base classes like SourceReaderBase to simplify implementation by handling threading and synchronization.  
* **SourceSplit:** This is a simple, serializable object that represents a finite chunk of work to be processed by a SourceReader. A split could be a file or a part of a file, a Kafka partition with a start and end offset, or, in the context of our Cosmos DB connector, a specific partition range of a container.

This architectural shift from a monolithic function to a decoupled coordinator/worker model is not merely an implementation detail; it reflects a fundamental evolution in distributed systems design. It is a direct application of the principle of separating the control plane (coordination, planning, discovery) from the data plane (execution, data movement). The SourceFunction conflated these two planes into a single run() method. FLIP-27, by contrast, explicitly separates them. The SplitEnumerator is the control plane component, running on the JobManager (the cluster's control plane) and managing the distribution of work. The SourceReader is the data plane component, running on the TaskManagers (the cluster's data plane) and executing the assigned work. This separation enables far more sophisticated coordination, such as locality-aware split assignment, more robust fault tolerance, as the state of the control plane can be checkpointed independently, and better overall resource utilization. Understanding this paradigm is the key to mastering the FLIP-27 API and building truly production-grade connectors.

### **2.3 Comparative Analysis: Why FLIP-27 is the Definitive Choice**

The advantages of the modern Source API over the legacy SourceFunction are substantial and decisive. The following table provides a clear comparison:

| Feature | Legacy SourceFunction API | Modern Source API (FLIP-27) |
| :---- | :---- | :---- |
| **Architecture** | Monolithic, single-threaded logic within run() | Decoupled master-slave (SplitEnumerator on master, SourceReader on workers) |
| **Parallelism** | Implicit, managed by Flink runtime; work distribution requires custom logic | Explicit, fine-grained control over work distribution via SourceSplits |
| **Split Discovery** | Not a native concept; must be implemented manually within run() | First-class citizen; handled centrally and statefully by SplitEnumerator |
| **Batch/Streaming Unification** | Primarily designed for streaming; batch support is an afterthought | Natively unified API for both bounded (batch) and unbounded (streaming) sources |
| **Event-Time Integration** | Manual watermark emission within the run() loop | Clean integration via WatermarkStrategy applied to the source |
| **Fault Tolerance** | Relies on the general CheckpointedFunction interface for state | Built-in, distinct, and clearer state management for both SplitEnumerator and SourceReaders |
| **Community Status** | Deprecated; scheduled for removal in Flink 2.0 | Actively developed, recommended, and the future standard for all Flink connectors |

The Flink community's direction is unambiguous: SourceFunction is a legacy component that should not be used for new development. Building a new connector on this deprecated API would be a significant technical misstep, accumulating technical debt from day one. While some developers have noted that the new API has a steeper learning curve and can seem "too complicated" for simple cases, this complexity is a direct reflection of the power and flexibility required to build robust, scalable, and unified data sources. This report aims to demystify that complexity and provide a clear path to implementation. For the Cosmos DB source connector, the FLIP-27 Source API is not just the better choice; it is the only viable choice for a production-grade implementation.

## **Section 3: Designing the Cosmos DB Source Connector**

With the FLIP-27 Source API established as the foundational framework, the next step is to translate its abstract components into a concrete architectural design tailored to the specifics of Azure Cosmos DB. This involves mapping the concepts of splits, enumeration, and reading to the primitives provided by the Cosmos DB Change Feed and its client SDK.

### **3.1 Architectural Blueprint: The FLIP-27 Model in a Cosmos DB Context**

The architecture of the Cosmos DB source connector will be a direct implementation of the FLIP-27 model. The key is to correctly define the role of each component in the context of consuming the Change Feed.

* **CosmosDBSource:** This will be the main entry-point class for the connector. It will implement the Source interface and will be responsible for configuration (e.g., Cosmos DB endpoint, key, database/container names) and for creating instances of the SplitEnumerator and the SourceReader.  
* **CosmosDBSplitEnumerator:** This component will be the central coordinator. Its primary role is to discover the physical partitions of the target Cosmos DB container. In the Cosmos DB SDK, these partitions are represented as FeedRange objects. The enumerator will wrap each FeedRange into a CosmosDBSplit and assign these splits to the available SourceReaders.  
* **CosmosDBSourceReader:** This is the parallel worker. Each instance will run on a TaskManager and will be responsible for the actual data ingestion. Upon receiving a CosmosDBSplit from the enumerator, it will instantiate and start the Cosmos DB Change Feed Processor (CFP) to begin consuming changes for the partition(s) represented by that split.  
* **CosmosDBSplit:** This is the serializable unit of work. It will encapsulate a Cosmos DB FeedRange. Critically, to support fault tolerance, it must also be able to store the state of consumption for that range. This state is represented by a continuation token provided by Cosmos DB.

A crucial design decision is how to manage the partition leases in the Cosmos DB lease container. The CFP has a sophisticated, built-in mechanism for coordinating these leases among multiple consumer instances, handling lease renewal, expiration, and acquisition automatically.4 Attempting to replicate this complex logic within the Flink

SplitEnumerator would be both redundant and highly error-prone. It would be fighting against the intended design of the Cosmos DB SDK.

Therefore, the optimal design is for the SplitEnumerator to adopt a "hands-off" approach to lease management. Its responsibility is to discover the complete set of partitions and perform an initial, high-level distribution of this work to the SourceReaders. The actual, dynamic load balancing and fine-grained lease acquisition will be handled by the fleet of CFP instances running inside the SourceReaders. When configured with the same leasePrefix, these CFP instances will automatically coordinate among themselves using the lease container to ensure each partition is processed by exactly one reader at a time. This design dramatically simplifies the Flink-side logic, correctly leverages the power of the Cosmos DB SDK, and aligns the Flink source with the recommended practices for Change Feed consumption.

### **3.2 The CosmosDBSplitEnumerator: Discovering and Distributing Work**

The CosmosDBSplitEnumerator is the brain of the operation, responsible for orchestrating the consumption process.

* **Initialization (start()):** Upon startup, the enumerator will use the CosmosAsyncClient from the Java SDK to connect to the target Cosmos DB account. Its first action will be to retrieve the list of all physical partitions for the monitored container. The SDK provides methods to get these as a list of FeedRange objects. The enumerator will then create a corresponding CosmosDBSplit for each FeedRange and store them in an internal collection of pending splits.  
* **Handling Reader Requests (handleSplitRequest()):** As SourceReader tasks come online, they will send split requests to the enumerator. The handleSplitRequest method will be invoked for each request. The enumerator's logic here is simple: if there are any pending splits in its internal collection, it will take one and assign it to the requesting reader.  
* **Handling Reader Failures (addSplitsBack()):** Fault tolerance is a key responsibility. If a SourceReader fails, Flink's runtime will detect this. Any splits that were assigned to that reader since the last successful checkpoint will be returned to the enumerator via the addSplitsBack method. The enumerator must add these splits back to its collection of pending splits so they can be reassigned to another, healthy reader, ensuring no partition is left unprocessed.

### **3.3 The CosmosDBSourceReader: Parallel Consumption of the Change Feed**

The CosmosDBSourceReader is where the data movement happens. Each parallel instance performs the following tasks:

* **Hosting the Change Feed Processor:** Upon receiving a CosmosDBSplit (or a list of them), the SourceReader's primary task is to configure and start an instance of the ChangeFeedProcessor. The processor will be configured to monitor the specific FeedRange contained within the split.  
* **Decoupling Push from Pull:** The CFP operates on a "push" model; it invokes a user-defined delegate method (handleChanges) whenever a new batch of documents is available.4 Flink's internal data processing model, however, is based on a "pull" model, where the  
  SourceOperator calls the SourceReader's pollNext() method to request data. A critical pattern for the connector is to bridge these two models. The handleChanges delegate will not emit data directly. Instead, it will place the received batch of documents into a thread-safe internal buffer (e.g., a java.util.concurrent.BlockingQueue).  
* **Emitting Records (pollNext()):** The pollNext() method, called by the Flink runtime, will then simply poll this internal buffer. If records are available, it will take them from the buffer and emit them into the Flink DataStream. This design decouples the I/O-bound work of the CFP from the Flink's main processing thread, which is a recommended best practice for preventing performance degradation and checkpointing issues.

### **3.4 Defining the CosmosDBSplit and its State**

The CosmosDBSplit is the message passed from the SplitEnumerator to the SourceReader, representing a unit of work. It is a simple, serializable data object. Its primary content is the identifier for the Cosmos DB FeedRange that the reader should process.

However, for fault tolerance, the state of processing for that split must also be captured. This state is the **continuation token**. A continuation token is an opaque string provided by Cosmos DB that acts as a bookmark, marking the precise position in the Change Feed from which consumption should resume. Therefore, the state associated with each split is simply its latest continuation token. This token is the most critical piece of information that needs to be managed and persisted to achieve fault-tolerant, exactly-once processing. The mechanism for persisting this state is Flink's checkpointing system, which is the focus of the next section.

## **Section 4: State Management and Fault Tolerance: The Checkpointing Nexus**

The cornerstone of a production-grade Flink connector is its ability to integrate seamlessly with Flink's checkpointing mechanism to provide strong fault tolerance guarantees. For the Cosmos DB source, this integration hinges on a single, critical concept: mapping the Cosmos DB continuation token to Flink's distributed state. This is the nexus where the fault tolerance models of both systems meet to provide end-to-end exactly-once semantics.

### **4.1 The Core Integration Pattern: Mapping Continuation Tokens to Flink State**

The fundamental principle of the connector's state management is as follows: **the latest continuation token for each processed partition (FeedRange) is the state that must be atomically persisted in a Flink checkpoint.**

When the Change Feed Processor (CFP) processes a batch of changes, it provides the corresponding continuation token that marks the end of that batch. This token is the precise bookmark needed to resume processing without missing data or re-processing already completed data.

To manage this, each CosmosDBSourceReader instance will maintain an internal, in-memory map: Map\<String, String\>, where the key is the FeedRange identifier and the value is the latest continuation token received for that range. As the handleChanges delegate of the CFP receives new batches of documents, it will update this map with the new continuation token for the relevant partition.

### **4.2 Implementing snapshotState: Persisting Progress During Checkpoints**

Flink's checkpointing is an active process initiated by the JobManager. Periodically, it injects special markers called "checkpoint barriers" into the data streams at the sources. When a CosmosDBSourceReader receives a checkpoint barrier, its snapshotState() method is invoked by the Flink runtime.

The implementation of this method is critical:

1. The snapshotState() method will access the internal Map of continuation tokens.  
2. It will serialize this map.  
3. It will then write the serialized map to the Flink ListState provided by the StateInitializationContext.

This action atomically saves the exact progress of each partition being processed by that reader into the distributed, fault-tolerant state backend configured for the Flink job (e.g., HDFS, S3). This snapshot is part of a global, consistent checkpoint of the entire Flink application's state.

### **4.3 Failure Recovery: How the Connector Restores State and Resumes Processing**

In the event of a failure (e.g., a TaskManager crash), Flink's recovery mechanism is triggered. The job is restarted, and each operator is restored to the state of the last successfully completed checkpoint.

For the CosmosDBSourceReader, this means:

1. A new SourceReader task is initialized on a healthy TaskManager.  
2. During its initialization phase, Flink calls its initializeState() method, providing it with the state that was saved during the last successful checkpoint.  
3. The reader's implementation of initializeState() will read the serialized data from the Flink state backend and deserialize it back into the Map\<String, String\> of continuation tokens.  
4. Now, when the reader receives a CosmosDBSplit to process, it will check its restored map for a corresponding continuation token.  
5. If a token exists, it will configure the ChangeFeedProcessorBuilder for that partition using the startFromContinuation() option, passing the restored token.

This ensures that the CFP does not start reading from the beginning of the partition's change feed, nor from the latest point, but from the exact position recorded in the last consistent global snapshot of the application. This mechanism is the key to preventing both data loss and data duplication upon recovery.

### **4.4 Ensuring Exactly-Once Semantics with the Change Feed Processor**

By combining the guarantees of the CFP with the mechanics of Flink's checkpointing, the connector can achieve end-to-end exactly-once processing semantics.

* The Cosmos DB Change Feed Processor itself provides an **"at-least-once"** delivery guarantee. If an error occurs within the delegate code, the CFP will retry processing the same batch of changes until it succeeds.4  
* Flink's checkpointing mechanism provides the **"exactly-once"** state management layer.

The synergy is achieved through the checkpoint barrier alignment. A Flink checkpoint is a globally consistent snapshot. It is initiated when the JobManager injects barriers into the sources. When our CosmosDBSourceReader receives a barrier, it snapshots its state (the continuation tokens). This barrier then flows downstream through the entire job graph. Every stateful operator it encounters (e.g., a window operator, a custom ProcessFunction) also snapshots its own state in alignment with that same barrier. The checkpoint is only considered complete once the barriers have traversed the entire graph and all operators have reported their state snapshots to the JobManager.

This means that the continuation token snapshotted at the source is causally and consistently linked to the state of every other operator in the pipeline at that precise moment in the stream. If a failure occurs, the *entire* job graph reverts to the state associated with the last successful checkpoint. The CosmosDBSourceReader restarts from its saved token, and all downstream operators restore their corresponding state. This guarantees that the entire pipeline is in a consistent state relative to the data that is about to be re-read from Cosmos DB, which is the essence of an end-to-end exactly-once guarantee. It is not merely about the source being correct in isolation, but about the source's state being consistent with the state of the entire pipeline.

## **Section 5: Java Implementation Deep Dive**

This section provides a complete, production-oriented implementation of the Cosmos DB source connector in Java. The code is designed to be robust, clear, and directly usable as a foundation for a real-world project.

### **5.1 Project Setup and Dependencies (Maven)**

To begin, set up a Maven project and include the necessary dependencies in your pom.xml. The core dependencies are Flink's DataStream API and the Azure Cosmos DB SDK for Java V4.

XML

\<properties\>  
    \<flink.version\>1.17.1\</flink.version\>  
    \<java.version\>11\</java.version\>  
    \<azure.cosmos.version\>4.49.0\</azure.cosmos.version\>  
\</properties\>

\<dependencies\>  
    \<dependency\>  
        \<groupId\>org.apache.flink\</groupId\>  
        \<artifactId\>flink-streaming-java\</artifactId\>  
        \<version\>${flink.version}\</version\>  
        \<scope\>provided\</scope\>  
    \</dependency\>  
    \<dependency\>  
        \<groupId\>org.apache.flink\</groupId\>  
        \<artifactId\>flink-clients\</artifactId\>  
        \<version\>${flink.version}\</version\>  
        \<scope\>provided\</scope\>  
    \</dependency\>

    \<dependency\>  
        \<groupId\>com.azure\</groupId\>  
        \<artifactId\>azure-cosmos\</artifactId\>  
        \<version\>${azure.cosmos.version}\</version\>  
    \</dependency\>

    \<dependency\>  
        \<groupId\>org.slf4j\</groupId\>  
        \<artifactId\>slf4j-simple\</artifactId\>  
        \<version\>1.7.36\</version\>  
        \<scope\>runtime\</scope\>  
    \</dependency\>  
\</dependencies\>

### **5.2 Implementing the CosmosDBSource and its Components**

The implementation follows the FLIP-27 architecture, comprising several key classes.

#### **5.2.1 CosmosDBSplit: The Unit of Work**

This is a simple, serializable class representing a FeedRange. It implements SourceSplit.

Java

import org.apache.flink.api.connector.source.SourceSplit;

public class CosmosDBSplit implements SourceSplit {

    private final String feedRange;  
    private final String splitId;

    public CosmosDBSplit(String feedRange) {  
        this.feedRange \= feedRange;  
        this.splitId \= feedRange; // Use the FeedRange itself as a unique ID  
    }

    public String getFeedRange() {  
        return feedRange;  
    }

    @Override  
    public String splitId() {  
        return splitId;  
    }  
}

#### **5.2.2 CosmosDBSplitSerializer: Handling Serialization**

Flink requires custom serializers for splits and enumerator state to avoid relying on Java serialization.

Java

import org.apache.flink.core.io.SimpleVersionedSerializer;  
// other imports

public class CosmosDBSplitSerializer implements SimpleVersionedSerializer\<CosmosDBSplit\> {

    @Override  
    public int getVersion() {  
        return 1;  
    }

    @Override  
    public byte serialize(CosmosDBSplit split) throws IOException {  
        try (ByteArrayOutputStream baos \= new ByteArrayOutputStream();  
             DataOutputStream out \= new DataOutputStream(baos)) {  
            out.writeUTF(split.getFeedRange());  
            out.flush();  
            return baos.toByteArray();  
        }  
    }

    @Override  
    public CosmosDBSplit deserialize(int version, byte serialized) throws IOException {  
        if (version \== 1) {  
            try (ByteArrayInputStream bais \= new ByteArrayInputStream(serialized);  
                 DataInputStream in \= new DataInputStream(bais)) {  
                String feedRange \= in.readUTF();  
                return new CosmosDBSplit(feedRange);  
            }  
        }  
        throw new IOException("Unknown version: " \+ version);  
    }  
}

#### **5.2.3 CosmosDBSource: The Main Entry Point**

This class ties everything together. It holds the configuration and creates the other components.

Java

import org.apache.flink.api.connector.source.\*;  
// other imports

public class CosmosDBSource\<OUT\> implements Source\<OUT, CosmosDBSplit, Map\<String, String\>\> {

    private final CosmosDBSourceConfig config;  
    private final DeserializationSchema\<OUT\> deserializer;

    public CosmosDBSource(CosmosDBSourceConfig config, DeserializationSchema\<OUT\> deserializer) {  
        this.config \= config;  
        this.deserializer \= deserializer;  
    }

    @Override  
    public Boundedness getBoundedness() {  
        return Boundedness.CONTINUOUS\_UNBOUNDED;  
    }

    @Override  
    public SplitEnumerator\<CosmosDBSplit, Map\<String, String\>\> createEnumerator(  
            SplitEnumeratorContext\<CosmosDBSplit\> enumContext) {  
        return new CosmosDBSplitEnumerator(enumContext, config, null);  
    }

    @Override  
    public SplitEnumerator\<CosmosDBSplit, Map\<String, String\>\> restoreEnumerator(  
            SplitEnumeratorContext\<CosmosDBSplit\> enumContext,  
            Map\<String, String\> checkpoint) {  
        return new CosmosDBSplitEnumerator(enumContext, config, checkpoint);  
    }

    @Override  
    public SimpleVersionedSerializer\<CosmosDBSplit\> getSplitSerializer() {  
        return new CosmosDBSplitSerializer();  
    }

    @Override  
    public SimpleVersionedSerializer\<Map\<String, String\>\> getEnumeratorCheckpointSerializer() {  
        // For simplicity, using a basic map serializer. A more robust implementation  
        // would handle this with custom serialization logic.  
        return new MapSerializer();  
    }

    @Override  
    public SourceReader\<OUT, CosmosDBSplit\> createReader(SourceReaderContext readerContext) {  
        return new CosmosDBSourceReader\<\>(readerContext, config, deserializer);  
    }  
}

### **5.3 Code Walkthrough: CosmosDBSplitEnumerator Logic**

The enumerator discovers and distributes the FeedRanges.

Java

import com.azure.cosmos.CosmosAsyncClient;  
// other imports

public class CosmosDBSplitEnumerator implements SplitEnumerator\<CosmosDBSplit, Map\<String, String\>\> {

    private final SplitEnumeratorContext\<CosmosDBSplit\> context;  
    private final CosmosDBSourceConfig config;  
    private final Set\<String\> pendingSplits;  
    private final Map\<Integer, Set\<String\>\> assignedSplits;

    public CosmosDBSplitEnumerator(  
            SplitEnumeratorContext\<CosmosDBSplit\> context,  
            CosmosDBSourceConfig config,  
            Map\<String, String\> checkpoint) {  
        this.context \= context;  
        this.config \= config;  
        this.assignedSplits \= new HashMap\<\>();  
        this.pendingSplits \= new HashSet\<\>();  
        // In a real implementation, the checkpoint would restore the state  
        // of pending and assigned splits.  
    }

    @Override  
    public void start() {  
        // Use the context's single-threaded executor for async operations  
        context.callAsync(  
            () \-\> {  
                // Discover partitions (FeedRanges) from Cosmos DB  
                try (CosmosAsyncClient client \= config.buildCosmosAsyncClient()) {  
                    return client.getDatabase(config.getDatabase())  
                       .getContainer(config.getMonitoredContainer())  
                       .getFeedRanges()  
                       .collectList()  
                       .toFuture();  
                }  
            },  
            (feedRanges, throwable) \-\> {  
                if (throwable\!= null) {  
                    // Handle failure  
                    return;  
                }  
                // Add discovered splits to the pending set  
                for (FeedRange range : feedRanges) {  
                    pendingSplits.add(range.toString());  
                }  
                // Check if any readers are already waiting for splits  
                assignPendingSplits();  
            }  
        );  
    }

    @Override  
    public void handleSplitRequest(int subtaskId, @Nullable String requesterHostname) {  
        // A reader is requesting work, assign if available  
        assignPendingSplitsToReader(subtaskId);  
    }

    @Override  
    public void addSplitsBack(List\<CosmosDBSplit\> splits, int subtaskId) {  
        // A reader failed, re-add its splits to the pending set  
        Set\<String\> readerAssigned \= assignedSplits.remove(subtaskId);  
        if (readerAssigned\!= null) {  
            pendingSplits.addAll(readerAssigned);  
        }  
    }

    //... other required methods (addReader, snapshotState, close)...  
      
    private void assignPendingSplits() {  
        for (int subtask : context.registeredReaders().keySet()) {  
            assignPendingSplitsToReader(subtask);  
        }  
    }

    private void assignPendingSplitsToReader(int subtaskId) {  
        if (\!pendingSplits.isEmpty()) {  
            String splitId \= pendingSplits.iterator().next();  
            pendingSplits.remove(splitId);  
              
            context.assignSplit(new CosmosDBSplit(splitId), subtaskId);  
              
            assignedSplits.computeIfAbsent(subtaskId, k \-\> new HashSet\<\>()).add(splitId);  
        }  
    }  
}

### **5.4 Code Walkthrough: CosmosDBSourceReader Integration with the Change Feed Processor**

This is the core worker logic, handling data ingestion and state management.

Java

import org.apache.flink.api.connector.source.SourceReaderContext;  
import org.apache.flink.connector.base.source.reader.SingleThreadMultiplexSourceReaderBase;  
// other imports

public class CosmosDBSourceReader\<OUT\> extends SingleThreadMultiplexSourceReaderBase\<  
        JsonNode, OUT, CosmosDBSplit, Map\<String, String\>\> {

    private final CosmosDBSourceConfig config;  
    private final DeserializationSchema\<OUT\> deserializer;  
    private final Map\<String, String\> continuationTokens;  
    private transient ChangeFeedProcessor changeFeedProcessor;

    public CosmosDBSourceReader(  
            SourceReaderContext readerContext,  
            CosmosDBSourceConfig config,  
            DeserializationSchema\<OUT\> deserializer) {  
        super(  
            () \-\> new CosmosDBSplitReader(config), // SplitReader factory  
            new CosmosDBRecordEmitter\<\>(deserializer), // RecordEmitter  
            readerContext.getConfiguration(),  
            readerContext);  
        this.config \= config;  
        this.deserializer \= deserializer;  
        this.continuationTokens \= new ConcurrentHashMap\<\>();  
    }

    @Override  
    public void start() {  
        // Request the first split  
        context.sendSplitRequest();  
    }

    @Override  
    protected void onSplitFinished(Map\<String, Map\<String, String\>\> finishedSplitStates) {  
        // Request a new split when the current one is done (for batch, not continuous)  
        context.sendSplitRequest();  
    }

    @Override  
    protected Map\<String, String\> initializedState(CosmosDBSplit split) {  
        // This method is called when a new split is assigned.  
        // We return the map of continuation tokens which becomes the state for this split.  
        return continuationTokens;  
    }

    @Override  
    protected CosmosDBSplit toSplitType(String splitId, Map\<String, String\> splitState) {  
        // Restore the split object from its ID and state  
        return new CosmosDBSplit(splitId);  
    }  
      
    @Override  
    public List\<CosmosDBSplit\> snapshotState(long checkpointId) {  
        // The state is managed by the SplitReader and emitted via RecordsWithSplitIds.  
        // The base class handles checkpointing this state.  
        return super.snapshotState(checkpointId);  
    }  
}

The CosmosDBSplitReader (not shown in full detail for brevity) would be responsible for initializing the ChangeFeedProcessor. Its fetch() method would poll the internal BlockingQueue populated by the CFP's delegate. The delegate's handleChanges function is where the magic happens:

Java

// Inside the ChangeFeedProcessor delegate implementation  
private void handleChanges(List\<JsonNode\> docs, ChangeFeedProcessorContext context) {  
    for (JsonNode doc : docs) {  
        // Add document to a thread-safe queue for the fetch() method to consume  
        recordsQueue.add(doc);  
    }  
    // IMPORTANT: Update the continuation token for checkpointing  
    String continuation \= context.getContinuationToken();  
    String feedRange \= context.getFeedRange();  
    continuationTokens.put(feedRange, continuation);  
}

The state (the continuationTokens map) is then passed from the SplitReader to the SourceReaderBase via the RecordsWithSplitIds object returned by fetch(), and the base class ensures it is checkpointed correctly.

### **5.5 Putting It All Together: A Complete, Executable Flink Job**

Finally, a main method demonstrates how to use the source.

Java

public class CosmosDBJob {  
    public static void main(String args) throws Exception {  
        StreamExecutionEnvironment env \= StreamExecutionEnvironment.getExecutionEnvironment();  
        env.enableCheckpointing(5000); // Enable checkpointing

        CosmosDBSourceConfig config \= new CosmosDBSourceConfig(  
            "YOUR\_COSMOS\_ENDPOINT",  
            "YOUR\_COSMOS\_KEY",  
            "your-database",  
            "your-monitored-container",  
            "your-lease-container"  
        );

        // Using a simple schema to convert JsonNode to String  
        DeserializationSchema\<String\> deserializer \= new SimpleStringSchema();

        CosmosDBSource\<String\> cosmosSource \= new CosmosDBSource\<\>(config, deserializer);  
          
        DataStream\<String\> stream \= env.fromSource(  
            cosmosSource,  
            WatermarkStrategy.noWatermarks(),  
            "CosmosDB Source"  
        );

        stream.print();

        env.execute("Flink Cosmos DB Source Job");  
    }  
}

This complete Java implementation provides a solid, fault-tolerant foundation for ingesting data from Cosmos DB into Flink, correctly leveraging the modern Source API and the Change Feed Processor.

## **Section 6: Scala Implementation Deep Dive**

This section provides a complete implementation of the Cosmos DB source connector using Scala. While Flink's core APIs are Java-based, Scala can be used effectively to create concise, type-safe, and idiomatic connectors. The focus here is on leveraging Scala's strengths while interoperating seamlessly with the Java-based Flink and Cosmos DB SDKs.

### **6.1 Navigating Flink's API Landscape with Scala**

It is important to note the evolution of Flink's Scala support. The original, dedicated Scala DataStream API (org.apache.flink.streaming.api.scala) has been deprecated and is scheduled for removal in Flink 2.0. The community's recommended approach is now to use the Java DataStream API directly from Scala code. This approach provides access to the latest Flink features and avoids reliance on a deprecated module.

While community projects like flink-scala-api exist to provide better Scala type class derivation for serialization, this guide will focus on using the standard Java API to ensure maximum compatibility and adherence to the official Flink roadmap. This requires some careful handling of type information and Java-Scala collections interoperability, but results in a robust and future-proof implementation.

### **6.2 Idiomatic Scala Patterns for Implementing the FLIP-27 Interfaces**

Scala's features can make the implementation of the FLIP-27 interfaces cleaner and more expressive.

* **Case Classes for Splits:** Scala case classes are a perfect fit for representing the CosmosDBSplit. They provide immutability, structural equality, and automatic generation of hashCode, equals, and toString methods, making them ideal for data-carrying objects.  
* **Functional Collections:** Using Scala's immutable collections (Set, Map) within the SplitEnumerator can help prevent side effects and make the state management logic easier to reason about.  
* **Option for Nullability:** Scala's Option type can be used to handle nullable values from the Java APIs more safely, such as the requesterHostname in handleSplitRequest.  
* **Java Interoperability:** The scala.jdk.CollectionConverters library is essential for converting between Scala and Java collections, which is necessary when interacting with the Flink and Cosmos DB SDKs.

### **6.3 Code Walkthrough: A Scala-centric Implementation of the Connector**

Here is the full implementation of the connector components in idiomatic Scala.

#### **6.3.1 CosmosDBSplit and its Serializer**

Using a case class for the split simplifies the definition.

Scala

import org.apache.flink.api.connector.source.SourceSplit

case class CosmosDBSplit(feedRange: String) extends SourceSplit {  
  override def splitId(): String \= feedRange  
}

import org.apache.flink.core.io.SimpleVersionedSerializer  
import java.io.\_

class CosmosDBSplitSerializer extends SimpleVersionedSerializer {  
  override def getVersion: Int \= 1

  override def serialize(split: CosmosDBSplit): Array \= {  
    val baos \= new ByteArrayOutputStream()  
    val out \= new DataOutputStream(baos)  
    out.writeUTF(split.feedRange)  
    out.flush()  
    baos.toByteArray  
  }

  override def deserialize(version: Int, serialized: Array): CosmosDBSplit \= {  
    if (version \== 1) {  
      val bais \= new ByteArrayInputStream(serialized)  
      val in \= new DataInputStream(bais)  
      val feedRange \= in.readUTF()  
      CosmosDBSplit(feedRange)  
    } else {  
      throw new IOException(s"Unknown version: $version")  
    }  
  }  
}

#### **6.3.2 CosmosDBSource: The Scala Entry Point**

The source implementation looks very similar to the Java version, but uses Scala types and syntax.

Scala

import org.apache.flink.api.connector.source.\_  
import org.apache.flink.api.common.serialization.DeserializationSchema  
import java.util.{Map \=\> JMap}

class CosmosDBSource(  
    config: CosmosDBSourceConfig,  
    deserializer: DeserializationSchema  
) extends Source\] {

  override def getBoundedness: Boundedness \= Boundedness.CONTINUOUS\_UNBOUNDED

  override def createEnumerator(  
      enumContext: SplitEnumeratorContext  
  ): SplitEnumerator\] \= {  
    new CosmosDBSplitEnumerator(enumContext, config, null)  
  }

  override def restoreEnumerator(  
      enumContext: SplitEnumeratorContext,  
      checkpoint: JMap  
  ): SplitEnumerator\] \= {  
    new CosmosDBSplitEnumerator(enumContext, config, checkpoint)  
  }

  override def getSplitSerializer: SimpleVersionedSerializer \= {  
    new CosmosDBSplitSerializer()  
  }

  override def getEnumeratorCheckpointSerializer: SimpleVersionedSerializer\] \= {  
    new MapSerializer() // Same as Java  
  }

  override def createReader(readerContext: SourceReaderContext): SourceReader \= {  
    new CosmosDBSourceReader(readerContext, config, deserializer)  
  }  
}

#### **6.3.3 CosmosDBSplitEnumerator in Scala**

The enumerator logic can be expressed more functionally using Scala collections.

Scala

import scala.collection.mutable  
import scala.jdk.CollectionConverters.\_  
import com.azure.cosmos.models.FeedRange

class CosmosDBSplitEnumerator(  
    context: SplitEnumeratorContext,  
    config: CosmosDBSourceConfig,  
    checkpoint: JMap // Checkpoint state for recovery  
) extends SplitEnumerator\] {

  private val pendingSplits: mutable.Set \= mutable.Set.empty  
  private val assignedSplits: mutable.Map\] \= mutable.Map.empty

  override def start(): Unit \= {  
    context.callAsync, Void\](  
      () \=\> {  
        val client \= config.buildCosmosAsyncClient()  
        client.getDatabase(config.getDatabase)  
         .getContainer(config.getMonitoredContainer)  
         .getFeedRanges()  
         .collectList()  
         .toFuture  
      },  
      (feedRanges, throwable) \=\> {  
        if (throwable\!= null) {  
          // Handle error  
        } else {  
          pendingSplits \++= feedRanges.asScala.map(\_.toString)  
          assignPendingSplits()  
        }  
      }  
    )  
  }

  override def handleSplitRequest(subtaskId: Int, requesterHostname: String): Unit \= {  
    assignPendingSplitsToReader(subtaskId)  
  }

  override def addSplitsBack(splits: java.util.List, subtaskId: Int): Unit \= {  
    assignedSplits.remove(subtaskId).foreach(pendingSplits \++= \_)  
  }  
    
  //... other required methods...

  private def assignPendingSplits(): Unit \= {  
    context.registeredReaders().keySet().asScala.foreach(assignPendingSplitsToReader)  
  }

  private def assignPendingSplitsToReader(subtaskId: Int): Unit \= {  
    if (pendingSplits.nonEmpty) {  
      val splitId \= pendingSplits.head  
      pendingSplits.remove(splitId)

      context.assignSplit(CosmosDBSplit(splitId), subtaskId)  
      assignedSplits.getOrElseUpdate(subtaskId, mutable.Set.empty).add(splitId)  
    }  
  }  
}

### **6.4 Leveraging Scala's Strengths for Concise and Type-Safe Connectors**

The CosmosDBSourceReader implementation in Scala would follow the same pattern as the Java version, extending SingleThreadMultiplexSourceReaderBase and bridging the CFP's push model with Flink's pull model via an internal queue.

While the overall structure remains dictated by the Flink APIs, Scala offers several advantages that lead to a more robust final implementation:

* **Immutability by Default:** Using val and immutable collections makes state management within the components safer and easier to reason about, reducing the risk of concurrent modification issues.  
* **Expressive Syntax:** Scala's syntax for anonymous functions and collection transformations (e.g., map, foreach) makes the logic more concise and readable compared to Java's more verbose equivalents.  
* **Type Safety:** The use of case classes and Scala's strong type system helps catch errors at compile time. For example, using Option for the continuation token can make it explicit that a token may not yet exist for a partition, preventing potential NullPointerExceptions.

By embracing these idiomatic Scala patterns, it is possible to build a Flink connector that is not only functionally correct and performant but also maintainable and robust, even when built on top of a Java-centric framework.

## **Section 7: Advanced Concepts and Operational Excellence**

Implementing a functional source connector is only the first step. A production-grade connector must also address advanced requirements such as event-time processing, performance tuning, robust monitoring, and schema evolution. This section covers these critical operational aspects, transforming the basic connector into a component ready for mission-critical deployments.

### **7.1 Event-Time Processing: Implementing a WatermarkStrategy**

For many stream processing applications, understanding the time at which events occurred is crucial. Flink's event-time processing allows for correct, deterministic results even when events arrive out of order. This is enabled by **watermarks**, which are special markers in the data stream that signal the progress of event time.

To enable event-time processing for the Cosmos DB source, a WatermarkStrategy must be applied when creating the DataStream.

Java

// Java Example  
DataStream\<String\> stream \= env.fromSource(  
    cosmosSource,  
    WatermarkStrategy  
       .\<String\>forBoundedOutOfOrderness(Duration.ofSeconds(10))  
       .withTimestampAssigner((event, timestamp) \-\> extractTimestamp(event)),  
    "CosmosDB Source with Watermarks"  
);

The WatermarkStrategy requires two components:

1. **A TimestampAssigner:** This is a function that extracts the event timestamp from each incoming record. For Cosmos DB documents, a common source for this timestamp is the \_ts property, which is a server-side, epoch-based timestamp of the last modification. The extractTimestamp function would parse the incoming JSON document and return the value of \_ts multiplied by 1000 to convert it to milliseconds.  
2. **A WatermarkGenerator:** This component is responsible for generating the watermarks themselves. Flink provides built-in generators, with forBoundedOutOfOrderness being the most common. This strategy assumes that events may arrive out of order, but only up to a specified maximum delay. It generates watermarks that are always t \- max\_delay, where t is the maximum event timestamp seen so far.

A potential issue with any source is **idleness**. If a particular partition in Cosmos DB receives no updates for a period, its corresponding SourceReader will not see any new events and will not advance its watermark. This can stall downstream time-based operations (like windows) for the entire application. To combat this, a custom WatermarkGenerator can be implemented that also advances the watermark based on processing time if no new events have been seen for a configurable idle timeout period.

### **7.2 Performance Tuning and RU Management: Avoiding Throttling and Optimizing Cost**

There exists a fundamental tension between Flink's operational goal—to process data as quickly as possible, maximizing throughput and minimizing latency—and Cosmos DB's service model, which is based on a provisioned throughput currency called **Request Units (RUs)**. Every operation against Cosmos DB, including reading from the Change Feed and updating the lease container, consumes RUs. If an application's requests consume more RUs per second than have been provisioned, Cosmos DB will throttle the requests, returning an HTTP 429 status code.

A naive Flink source, driven by the framework's desire to pull data aggressively, could easily overwhelm the provisioned RUs, especially in a highly parallel job. This would lead to constant throttling, increased latency, and potential job instability. Therefore, a production-ready connector must be an intelligent governor, actively managing this tension.

**Strategies for RU Management:**

* **Configure Polling Delay:** The ChangeFeedProcessorBuilder in the Cosmos DB SDK offers a options.setFeedPollDelay() (Java) or withFeedPollDelay() method. This setting controls how long the processor waits between polling the Change Feed for new changes if the previous poll returned no results. Setting a reasonable delay (e.g., 100ms to 1 second) is the first line of defense against overly aggressive polling.  
* **Monitor Normalized RU Consumption:** Azure Monitor provides a critical metric called Normalized RU Consumption %. This metric shows the utilization of the hottest physical partition in your container, on a per-second basis. Consistently high values (e.g., \> 80%) or frequent spikes to 100% are clear indicators that the Flink source is pushing the limits of the provisioned throughput and that throttling is likely occurring. This metric is essential for identifying hot partitions and for capacity planning.  
* **Capacity Planning:** The choice of throughput mode for your Cosmos DB containers is critical.  
  * **Provisioned Throughput:** Offers predictable performance but requires careful capacity planning. It is suitable for workloads with consistent traffic.  
  * **Autoscale:** Automatically scales the provisioned RUs within a configured range based on demand. This is well-suited for variable or unpredictable workloads but can be more expensive if not tuned correctly.  
  * Serverless: A consumption-based model where you pay per operation. It is ideal for intermittent or low-traffic workloads but can become expensive at high scale.  
    The Flink source's consumption pattern must be factored into this decision.  
* **Client-Side Rate Limiting:** For extreme cases, a client-side rate limiter (e.g., from the Guava library) could be implemented within the CosmosDBSplitReader to enforce a hard cap on the rate of requests sent to Cosmos DB.  
* **Graceful Error Handling:** The CFP delegate should be implemented to catch exceptions related to throttling (e.g., CosmosException with status code 429\) and implement a custom retry strategy with exponential backoff.

### **7.3 Monitoring and Observability: Key Metrics for Connector Health**

Effective monitoring is non-negotiable for production systems. The connector should expose key metrics using Flink's metrics system to provide visibility into its health and performance.

* **Custom Flink Metrics:** The CosmosDBSourceReader should register and update custom metrics. These can be exposed via a MetricGroup obtained from the SourceReaderContext. Essential metrics include:  
  * Gauge\<Long\> currentLag: The estimated number of changes remaining to be processed. This can be derived from the Cosmos DB Change Feed Estimator.  
  * Counter documentsEmitted: A simple counter for the total number of documents emitted.  
  * Meter documentsEmittedPerSecond: The rate of document emission.  
  * Gauge\<Long\> millisSinceLastRecord: The time in milliseconds since the last record was emitted, useful for detecting idleness.  
* **Change Feed Estimator:** The Cosmos DB SDK includes a Change Feed Estimator library. This tool can be run as a separate process or integrated into the SplitEnumerator to periodically estimate the number of changes that are yet to be read by the consumers (the Flink source readers). This provides a crucial measure of consumer lag.

### **7.4 Handling Schema Evolution and Data Deserialization**

Cosmos DB is a schemaless database, which means the structure of documents within a container can change over time. The Flink source and the downstream application must be resilient to such changes.

* **Deserialization:** The DeserializationSchema provided to the CosmosDBSource is responsible for converting the raw JsonNode or byte from Cosmos DB into a typed object (e.g., a POJO or a Flink Row). Using a robust library like Jackson is recommended.  
* **Schema Evolution Strategies:**  
  * **Tolerant Reader Pattern:** The deserialization logic should be designed to be tolerant of missing or extra fields. When deserializing to a POJO, Jackson can be configured to ignore unknown properties.  
  * **Generic Data Structures:** For maximum flexibility, the data can be deserialized into a more generic structure like Flink's Row or even kept as a raw JSON string for downstream operators to parse. This pushes the responsibility of handling schema to later stages of the pipeline but makes the source itself more resilient.  
  * **Schema Registry:** For more formal schema management, a schema registry (like Confluent Schema Registry) could be used, although this adds another component to the architecture. Documents in Cosmos DB would need to conform to a registered schema, and the Flink deserializer would use the registry to interpret the data correctly.

By addressing these advanced topics, the custom connector evolves from a simple data pipe into a robust, manageable, and observable component of a production-grade real-time data platform.

## **Section 8: Conclusion and Future Directions**

### **8.1 Summary of Key Architectural Decisions and Best Practices**

This report has provided a comprehensive architectural blueprint for building a production-grade Apache Flink source connector for Azure Cosmos DB. The design is rooted in a set of key decisions and best practices that ensure robustness, performance, and fault tolerance.

The core recommendations can be summarized as follows:

1. **Adopt the Modern Source API (FLIP-27):** The legacy SourceFunction is deprecated and lacks the necessary features for a modern, unified connector. The FLIP-27 Source API, with its decoupled SplitEnumerator (coordinator) and SourceReader (worker) architecture, provides the correct foundation for building a scalable and maintainable source.  
2. **Leverage the Change Feed Processor (CFP):** Instead of implementing low-level Change Feed consumption logic, the connector should build upon the official Cosmos DB SDK's Change Feed Processor. The CFP provides essential, out-of-the-box features for fault tolerance, state management, and dynamic load balancing, dramatically simplifying the connector's implementation.4  
3. **Map Continuation Tokens to Flink State:** The cornerstone of the connector's exactly-once guarantee is the precise mapping of Cosmos DB's continuation tokens to Flink's checkpointed state. By snapshotting the latest continuation token for each partition as part of Flink's global checkpointing mechanism, the connector can resume from the exact point of failure without data loss or duplication.  
4. **Proactively Manage Request Unit (RU) Consumption:** A production connector cannot be a naive data pipe. It must act as an intelligent governor that respects Cosmos DB's RU-based throughput limits. This involves configuring polling delays, monitoring for throttling (HTTP 429 errors), and aligning Flink's parallelism with the provisioned capacity of the Cosmos DB container to ensure stable, cost-effective operation.

By adhering to these principles, developers can construct a Flink-Cosmos DB source that is not only functional but also resilient and operationally sound, capable of powering mission-critical, real-time data pipelines.

### **8.2 Potential Enhancements**

While the architecture described provides a complete and robust foundation, there are several avenues for future enhancement that could further improve the connector's intelligence and adaptability.

* **Dynamic Split Discovery and Handling:** The current design discovers Cosmos DB partitions (FeedRanges) at the start of the job. While partition splits are not a frequent occurrence in Cosmos DB, a highly elastic container might undergo a split during the lifetime of a long-running Flink job. An advanced SplitEnumerator could be designed to periodically query Cosmos DB for the current set of FeedRanges. If it detects a split (i.e., a parent FeedRange is replaced by two children), it could gracefully signal the SourceReader processing the parent to finish and then add the new children splits to the pending queue for assignment. This would make the connector fully adaptive to the physical topology changes of the underlying Cosmos DB container.  
* **Adaptive RU Management and Backpressure:** The current model for RU management relies on static configuration (e.g., feedPollDelay). A more sophisticated implementation could create a self-tuning source. The SourceReader, upon detecting HTTP 429 (throttling) responses from Cosmos DB, could automatically and dynamically increase its internal polling delay or reduce its client-side rate limit. It could expose metrics about the frequency of throttling events, allowing the SplitEnumerator or an external control plane to make more global decisions. This would create a closed-loop control system where the connector automatically adapts its consumption rate to the available Cosmos DB capacity, maximizing throughput without causing instability.

These potential enhancements represent the next frontier in building intelligent, cloud-native connectors. They move beyond simple data integration towards creating components that are deeply aware of and responsive to the operational characteristics of the managed services they interact with, paving the way for truly autonomous and resilient data processing systems.

#### **Works cited**

1. microsoft/kafka-connect-cosmosdb: Kafka Connect ... \- GitHub, accessed August 29, 2025, [https://github.com/microsoft/kafka-connect-cosmosdb](https://github.com/microsoft/kafka-connect-cosmosdb)  
2. Work with the Change Feed \- Azure Cosmos DB | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/change-feed](https://learn.microsoft.com/en-us/azure/cosmos-db/change-feed)  
3. Read Azure Cosmos DB Change Feed | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/read-change-feed](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/read-change-feed)  
4. Change Feed Processor in Azure Cosmos DB | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/change-feed-processor](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/change-feed-processor)