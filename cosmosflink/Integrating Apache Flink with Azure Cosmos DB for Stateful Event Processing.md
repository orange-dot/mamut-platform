

# **A Comprehensive Guide to Integrating Apache Flink with Azure Cosmos DB for NoSQL: Custom Sink Development and Architectural Analysis**

## **Executive Summary**

### **Objective**

This report provides an exhaustive technical analysis and implementation guide for integrating Apache Flink with Azure Cosmos DB for NoSQL, focusing on the development of a custom data sink. It explores the critical architectural decisions, implementation strategies across Java, Python, and Scala, and best practices for building a robust, high-performance, and fault-tolerant data pipeline. The objective is to equip data engineering and architecture teams with the necessary knowledge to select the optimal integration pattern and successfully implement a production-grade solution for stateful, event-driven applications.

### **Key Findings**

Apache Flink's core strength lies in its sophisticated state management, making it an ideal framework for building complex, event-driven applications that require consistency and fault tolerance.1 This capability is central to use cases like fraud detection, real-time analytics, and business process monitoring, where processing an event requires context from previous events.1 The foundation of Flink's resilience is its checkpointing mechanism, which periodically creates consistent snapshots of the application's state and stores them in durable remote storage.1

Two primary architectural patterns emerge for writing data from Flink to Cosmos DB: an indirect pathway leveraging Apache Kafka and the Kafka Connect framework, and a direct pathway involving a custom Flink sink. The selection between these patterns represents a strategic trade-off between leveraging a mature, configuration-driven ecosystem and achieving maximum performance and control through custom development.

For organizations choosing the direct path, this report establishes that the combination of Flink's modern AsyncSinkBase API and the Azure Cosmos DB Java SDK v4 provides the definitive foundation for a resilient and high-throughput connector. This approach abstracts away common sink complexities like buffering and, crucially, integrates with Flink's checkpointing to ensure no data is lost on failure.4

A critical aspect of any stateful stream processing pipeline is the guarantee of data integrity. This analysis concludes that achieving end-to-end exactly-once semantics with Cosmos DB is most pragmatically accomplished through the implementation of idempotent write operations (specifically, upsert). While Flink's two-phase commit protocol is the canonical solution for transactional sinks, the partition-scoped nature of Cosmos DB's transactional batch feature makes a direct implementation complex. An idempotent sink, built upon an at-least-once delivery mechanism like AsyncSinkBase, offers a simpler, more idiomatic, and equally effective solution for ensuring data correctness.4

This report provides detailed implementation blueprints for Java, which serves as the reference architecture. It further details how this Java-based sink can be seamlessly integrated into Python (PyFlink) and Scala applications, acknowledging that due to PyFlink's architecture, any custom sink must be implemented on the JVM.

### **Core Recommendation**

For data pipelines where low end-to-end latency, fine-grained control over write behavior, and sophisticated backpressure management are paramount, developing a **direct custom sink is the superior architectural choice**. This approach eliminates intermediaries, reduces operational complexity by minimizing the number of distinct systems, and allows for optimal performance tuning tailored to the specific characteristics of both Flink and Cosmos DB.

Conversely, for teams that prioritize ecosystem maturity, a configuration-driven deployment model, and can tolerate slightly higher latency, the **indirect pathway via Kafka Connect remains a viable alternative**. This pattern benefits from a Microsoft-supported, feature-rich connector but introduces the operational overhead of managing a Kafka Connect cluster.

Ultimately, the recommended path for a custom sink is to build a Java implementation founded on Flink's AsyncSinkBase and the Azure Cosmos DB Java SDK v4, employing idempotent upsert operations to guarantee exactly-once semantics. This Java artifact can then serve as a robust and reusable component across all JVM-based Flink deployments, including those written in Scala and Python.

## **The Core of Flink: Stateful Stream Processing for Event-Driven Architectures**

Apache Flink is fundamentally designed for stateful stream processing, a paradigm that is essential for modern, event-driven applications.1 Unlike stateless processing, where each event is handled in isolation, stateful processing allows an application to remember information from past events and use that context to process current and future events.2 This capability is the cornerstone of advanced use cases such as fraud detection (tracking user behavior over time), anomaly detection (identifying deviations from a baseline), and real-time business process monitoring.1

### **Flink's Stateful Architecture**

Flink's architecture is built to handle state efficiently and reliably. It co-locates data and computation, enabling low-latency access to state information that can be stored in memory or on local disk.1 This design avoids the performance bottlenecks of constantly querying a remote database for context. The true power of Flink is unlocked through a combination of three core components: managed state, checkpointing, and event-time processing.2

#### **Key Components for Stateful Processing**

* **Managed State:** Flink takes direct control over managing the state of an application. For each keyed stream (data partitioned by an identifier like a user ID or device ID), Flink maintains a separate state.2 This state is automatically snapshotted as part of Flink's fault-tolerance mechanism and can be recovered in case of failure. Flink offers different state backends, such as in-memory for smaller state sizes and RocksDB, an embedded key-value store, for managing very large states that can exceed available memory.3  
* **Checkpointing for Fault Tolerance:** The core of Flink's reliability and its ability to provide exactly-once processing guarantees is its checkpointing mechanism.2 A checkpoint is a consistent, global snapshot of the application's state at a specific point in time.5 Flink's JobManager coordinates these checkpoints, instructing data sources to inject special markers called "checkpoint barriers" into the data stream. These barriers flow through the application graph, triggering each operator to snapshot its state. These state snapshots are then written to a durable, remote storage system like Amazon S3 or HDFS.1 In the event of a failure, Flink can restore the application from the last completed checkpoint, ensuring that no data is lost and no event is processed twice.1  
* **Event-Time Processing:** Flink has first-class support for event-time semantics, which means it processes events based on the timestamps embedded within the events themselves, rather than the time they arrive at the processor.1 This is crucial for handling out-of-order data, a common issue in distributed systems. Combined with event-time timers, this allows developers to implement complex, time-sensitive business logic, such as detecting when a user has been inactive for a specific period.2

### **Impact on Sink Design**

Because Flink's stateful processing and fault tolerance are built around checkpoints, any custom sink that writes data to an external system like Cosmos DB must integrate with this mechanism.4 A sink that is not "checkpoint-aware" cannot provide end-to-end guarantees. For example, if a job fails after the sink has written data but before the corresponding checkpoint is complete, a simple recovery would cause the same data to be written again, leading to duplicates. Therefore, a robust sink must coordinate its writes with Flink's checkpoints, either through transactional protocols or by ensuring idempotent write operations, to achieve true end-to-end exactly-once semantics.4

## **Architectural Pathways for Flink-to-Cosmos DB Integration**

The foundational decision in designing a data pipeline from Apache Flink to Azure Cosmos DB is the architectural pattern for data movement. This choice dictates not only the development effort but also the operational characteristics of the system, including latency, complexity, and cost. Two primary pathways exist: an indirect route that leverages the mature Apache Kafka ecosystem, and a direct route that requires the development of a custom Flink component.

### **Analysis of the Indirect Path: Flink \-\> Apache Kafka \-\> Kafka Connect Sink**

This architectural pattern decouples the Flink processing application from the Cosmos DB writer by introducing Apache Kafka as a durable, high-throughput message bus. In this model, the Flink job's sole responsibility as a producer is to serialize its results and write them to a designated Kafka topic. A separate, independently managed Kafka Connect cluster is then deployed to run the official Azure Cosmos DB Sink Connector. This connector polls the Kafka topic for new messages and writes them to the specified Cosmos DB container.7

#### **Pros**

* **Ecosystem Maturity and Support:** This approach leverages the robust, well-documented, and officially supported Azure Cosmos DB Sink Connector, available for both self-managed Kafka Connect and fully-managed services like Confluent Cloud.7 This significantly reduces the risk and development burden associated with building a custom solution.  
* **Operational Decoupling:** A key advantage is the separation of concerns. The Flink stream processing cluster and the Kafka Connect data ingestion cluster can be scaled, managed, and monitored independently. A failure or performance degradation in the sink connector (e.g., due to Cosmos DB throttling) will not directly cause the Flink job to fail; records will simply accumulate in the Kafka topic, providing a natural buffer and enhancing overall system resilience. Kafka itself provides significant advantages as a message bus, including durability, replayability, and the ability to serve multiple downstream consumers.19  
* **Configuration over Code:** The sink connector is managed primarily through declarative JSON configuration files. This allows for rapid deployment and modification of data pipelines without requiring extensive custom coding. Parameters such as connection endpoints, keys, topic-to-container mappings, and write strategies are defined in a configuration file that is submitted to the Kafka Connect REST API.7  
* **Rich Feature Set:** The official connector provides a wealth of built-in features. It supports various data formats, including plain JSON, JSON with Schema, and Avro, integrating with schema registries like Confluent Schema Registry.7 It also allows for the use of Single Message Transforms (SMTs), which can perform lightweight, in-flight modifications to messages before they are written to Cosmos DB. A common use case is the  
  InsertUuid SMT, which can automatically add an id field to documents that lack one.7  
* **Guaranteed Semantics:** The sink connector fully supports exactly-once semantics, a critical feature for maintaining data integrity in mission-critical applications. This ensures that each message from the Kafka topic is written to Cosmos DB precisely one time, even in the event of connector task failures and restarts.8

#### **Cons**

* **Increased End-to-End Latency:** The most significant drawback of this pattern is the introduction of additional latency. Data must be written to Kafka by Flink, committed, and then polled and read by the Kafka Connect task before finally being written to Cosmos DB. Each step adds network hops and processing time, making this architecture less suitable for use cases with stringent low-latency requirements.  
* **Higher Operational Complexity and Cost:** While the components are decoupled, the overall system architecture is more complex. It requires the deployment, management, monitoring, and maintenance of an additional distributed system: the Kafka Connect cluster.8 This entails infrastructure costs for the virtual machines or containers running the Connect workers, as well as the operational overhead of ensuring its health and performance.  
* **Limited Customization:** The configuration-driven nature of Kafka Connect, while an advantage for standard use cases, becomes a limitation when complex custom logic is required. Implementing sophisticated error handling, dynamic routing of records to different containers based on payload content, or highly customized backpressure strategies is difficult or impossible without developing custom SMTs or forking the connector itself.

### **Analysis of the Direct Path: Building a Custom Apache Flink Sink**

In this architectural pattern, the Flink job communicates directly with the Azure Cosmos DB service. The logic for connecting, batching, writing, and handling errors is encapsulated within a custom sink function that runs as part of the Flink job on the TaskManager nodes. This creates a more streamlined data flow, eliminating the Kafka intermediary for this specific data path.

#### **Pros**

* **Minimal Latency:** This architecture provides the most direct path from Flink's stream processing logic to Cosmos DB storage. By eliminating the intermediate Kafka topic and the Kafka Connect polling mechanism, it significantly reduces end-to-end latency, making it the preferred choice for real-time and near-real-time applications.  
* **Simplified Infrastructure:** The overall system topology is simpler, as it removes the need for a Kafka Connect cluster. This reduces the number of moving parts to manage, monitor, and secure in a production environment, potentially lowering both infrastructure and operational costs.  
* **Fine-Grained Control and Optimization:** A custom sink offers complete control over the interaction with Cosmos DB. Developers can implement bespoke logic for batching records to optimize Request Unit (RU) consumption, create sophisticated retry policies for handling transient errors like throttling (HTTP 429), and implement custom backpressure mechanisms that are tightly integrated with Flink's own stream processing backpressure signals.25 This allows for performance tuning that is precisely tailored to the application's workload and Cosmos DB's performance characteristics.  
* **Potentially Lower Cost:** By avoiding the infrastructure, licensing (if applicable), and operational costs associated with a dedicated Kafka cluster and Kafka Connect deployment, this approach can be more cost-effective, especially for pipelines where Kafka is not otherwise required.

#### **Cons**

* **Significant Development and Maintenance Overhead:** The primary drawback is the engineering effort required. The team is responsible for designing, building, thoroughly testing, and maintaining a production-grade connector. This is a non-trivial task that requires deep expertise in both Apache Flink's APIs and the Azure Cosmos DB SDK.  
* **Replication of Core Features:** The development team must implement features that are provided out-of-the-box by the Kafka Connect ecosystem. This includes robust fault tolerance through Flink's checkpointing mechanism, ensuring exactly-once semantics, creating a flexible and secure configuration system, and implementing comprehensive logging and metrics for observability.  
* **Tighter Coupling:** The Flink job is now directly coupled to the availability and performance of the Cosmos DB service. A poorly implemented sink with inadequate error handling or retry logic can cause instability in the entire Flink job, leading to frequent restarts or backpressure that stalls the entire pipeline. The resilience of the system is now entirely dependent on the quality of the custom sink code.

### **Comparative Analysis and Decision Framework**

The choice between the indirect and direct pathways is a fundamental architectural decision that hinges on a trade-off between leveraging a pre-built, feature-rich component versus achieving maximum performance and control through custom engineering. This is not merely a technical decision but a strategic one that must consider the project's specific non-functional requirements, the team's existing skill set, and the long-term operational model. While the Kafka Connect path appears more complex due to the additional system, for teams already operating a mature Kafka platform, it can be the simpler option from a development perspective.29 Conversely, the direct sink, while simpler architecturally, places a greater burden of complexity on the development team.

The following table provides a structured framework to guide this decision-making process:

| Feature/Metric | Direct Custom Sink | Indirect via Kafka Connect |
| :---- | :---- | :---- |
| **End-to-End Latency** | Very Low (Direct connection) | Moderate (Adds Kafka \+ Connect hops) |
| **Max Throughput** | High (Tunable batching/concurrency) | High (Scalable) but limited by polling |
| **Development Complexity** | High (Requires custom implementation) | Low (Configuration-based) |
| **Operational Complexity** | Low (Fewer systems to manage) | High (Requires Kafka \+ Connect cluster) |
| **Infrastructure Cost** | Lower (No Kafka/Connect cluster) | Higher (Cost of Kafka/Connect infra) |
| **Fault Tolerance Model** | Tightly coupled with Flink job | Decoupled; Kafka acts as a buffer |
| **Customization Flexibility** | Very High (Full control over logic) | Limited (Primarily via SMTs) |
| **Ecosystem Support** | Community/In-house | Officially supported by Microsoft |

## **Designing a High-Performance Custom Flink Sink**

Once the decision is made to pursue the direct architectural path, the focus shifts to designing a custom sink that is performant, resilient, and maintainable. A well-designed sink must adhere to core principles of modern distributed systems and correctly leverage the APIs provided by both Apache Flink and Azure Cosmos DB.

### **Core Principles: Batching, Asynchronicity, and Backpressure Management**

A naive sink implementation that performs a synchronous, single-item write for every record it receives will fail catastrophically under any significant load. A production-grade sink must be built on three foundational principles:

1. **Batching:** Azure Cosmos DB operations are measured in Request Units (RUs). Sending individual write requests for each record is highly inefficient, incurring significant RU costs and network overhead. The sink must buffer incoming records and group them into batches to be sent in a single request. The Azure Cosmos DB Java SDK v4 offers two primary mechanisms for this:  
   * **Bulk Operations:** The executeBulkOperations method on the CosmosAsyncContainer object is designed for high-throughput, non-transactional writes. It takes a stream of operations (e.g., createItem, upsertItem) and executes them in an optimized manner, transparently handling batching and concurrency to maximize throughput.32 This is the ideal choice for most Flink sink use cases.  
   * **Transactional Batch:** The executeCosmosBatch method allows for the atomic execution of a group of operations against a **single logical partition key**.35 This ensures that all operations in the batch either succeed or fail together. While powerful, its single-partition constraint makes it less suitable as a general-purpose mechanism in a parallel Flink sink, which will process data for many different partition keys simultaneously.  
2. **Asynchronicity:** All interactions with external systems like Cosmos DB must be asynchronous. Synchronous (blocking) I/O calls within a Flink operator will stall the processing thread, preventing it from handling other records or participating in checkpointing. This leads to a dramatic drop in throughput and can cause cascading failures. The sink must use an asynchronous client, such as the CosmosAsyncClient from the Azure Cosmos DB Java SDK v4, and leverage reactive programming paradigms (like Project Reactor's Flux and Mono) to handle I/O operations without blocking.34  
3. **Backpressure Management:** The sink acts as a crucial bridge between two sophisticated distributed systems, each with its own flow control mechanisms. Flink employs a credit-based backpressure system that propagates slowdowns from a slow sink operator back up the pipeline to the source.25 Cosmos DB signals that it is overloaded by rejecting requests with an HTTP 429 "Request Rate Too Large" status code.28 A well-designed sink must translate these 429 responses into Flink's backpressure mechanism. When Cosmos DB throttling is detected, the sink should slow down its consumption of new records from upstream operators, allowing the system to stabilize gracefully.

### **Choosing the Right Flink Sink API**

Apache Flink provides several APIs for building custom sinks. The choice of API has significant implications for the complexity of the implementation and the fault-tolerance guarantees it can provide.

* **The Modern AsyncSinkBase:** Introduced in Flink 1.15, the AsyncSinkBase is an abstract base class specifically designed for building high-throughput, at-least-once sinks for cloud services that do not have native two-phase commit capabilities.39 It provides a robust, pre-built framework that handles the most complex aspects of sink implementation:  
  * Buffering incoming records based on size, count, or time.  
  * Managing a queue of in-flight asynchronous requests.  
  * Applying backpressure when the number of in-flight requests reaches a configured limit.  
  * Persisting buffered, un-flushed records to Flink's state backend during checkpointing, ensuring no data is lost on failure.  
  * Retrying failed requests.

By using AsyncSinkBase, the developer is freed from implementing this complex boilerplate logic and can focus solely on the destination-specific implementation: converting Flink records into requests, submitting them to the destination, and interpreting the responses. This is the **highly recommended** API for a Cosmos DB sink.

* **The TwoPhaseCommitSinkFunction:** This is Flink's classic API for implementing sinks that provide end-to-end exactly-once semantics.41 It is based on the two-phase commit protocol and requires the destination system to support transactions that can be prepared (pre-committed) and then later either either committed or aborted. While powerful, it is more complex to implement and, as discussed below, is not the most natural fit for Cosmos DB's transactional capabilities.

### **Achieving Exactly-Once Semantics with Cosmos DB**

Providing exactly-once processing guarantees is a critical requirement for many data pipelines, ensuring that each input event affects the final state in Cosmos DB precisely one time, even in the presence of failures.

The canonical approach in Flink for this is the TwoPhaseCommitSinkFunction. This function coordinates with Flink's checkpointing mechanism. When a checkpoint begins (the "pre-commit" phase), the sink flushes its data and prepares a transaction with the external system. Once the checkpoint is successfully completed globally across the Flink job, the JobManager notifies the sinks to proceed with the "commit" phase, making the data visible.41

However, applying this pattern directly to Cosmos DB presents a significant challenge. Cosmos DB's TransactionalBatch feature, which provides ACID guarantees, is strictly scoped to operations within a single logical partition key.35 A parallel Flink sink (with a parallelism greater than one) will have multiple subtasks, each processing records for many different partition keys concurrently. There is no native Cosmos DB mechanism to coordinate a single distributed transaction across multiple partition keys, let alone across multiple Flink subtasks. Attempting to build such a coordination layer on top of

TransactionalBatch would be exceptionally complex and likely inefficient.

A far more pragmatic and idiomatic approach is to leverage idempotent writes. This pattern combines a sink that provides at-least-once delivery guarantees with a write operation that is safe to be repeated. If Flink fails and recovers from a previous checkpoint, it will re-process and re-send some records to the sink. An idempotent write ensures that this re-sending does not result in duplicate data or an incorrect state.

* Strategy 1 (Recommended): Idempotent Writes with AsyncSinkBase  
  This is the recommended strategy for achieving "effective" exactly-once semantics with Cosmos DB.  
  1. **At-Least-Once Delivery:** Use the AsyncSinkBase API. Flink's checkpointing mechanism, combined with the sink's ability to snapshot its internal buffers, guarantees that every record will be delivered to the Cosmos DB writer logic at least once.44  
  2. **Idempotent Operation:** In the sink's write logic, use the upsertItem operation provided by the Cosmos DB SDK.20 The  
     upsert operation is inherently idempotent: given a document, it will create the document if one with the same id and partition key does not exist, or it will replace the existing document if it does.  
  3. **Requirement:** This strategy mandates that every record flowing from Flink has a unique key that can be mapped to the id field in the Cosmos DB document. This key ensures that replayed records correctly overwrite their previous incarnations, leading to a consistent and correct final state.  
* Strategy 2 (Advanced/Not Recommended): Transactional Writes with TwoPhaseCommitSinkFunction  
  This approach is theoretically possible but practically challenging and offers little benefit over the idempotent method for this specific use case. It would require managing a map of open TransactionalBatch objects within each sink subtask, one for each partition key. The preCommit phase would involve executing all these batches, and the commit phase would be a no-op, as executeCosmosBatch is atomic. This adds significant state management complexity to the sink for no real gain in correctness over the simpler idempotent pattern.

### **Security and Configuration Management**

Managing credentials, such as the Cosmos DB endpoint and primary key, is a critical security concern. Hardcoding secrets into application code is unacceptable in a production environment.

Modern cloud-native deployment practices advocate for using external secret management systems. When deploying Flink on Kubernetes, the standard approach is to store the Cosmos DB credentials in a Kubernetes Secret. This secret can then be mounted into the Flink TaskManager pods as environment variables or files.49 The Flink application code, specifically the custom sink, would then be configured to read these credentials from the environment at startup.

While Flink's own configuration system (flink-conf.yaml) can be used and will automatically obscure values for keys containing "password" or "secret" in logs and the UI, this still requires placing the secret in a configuration file.52 Using a dedicated secrets management system provides better separation of concerns, auditing, and lifecycle management. Managed Flink platforms like Ververica Platform or Confluent Cloud offer their own integrated secret management abstractions, which should be the preferred method when using those platforms.53 The

ParameterTool utility in Flink can be used to read configuration from program arguments or properties files, which can be populated from environment variables injected by the secret store.52

## **Implementation in Java: The Reference Architecture**

This section provides a detailed, production-grade guide for implementing the custom Cosmos DB sink in Java. The implementation will be based on the recommended design principles: using Flink's AsyncSinkBase API for the framework, the Azure Cosmos DB Java SDK v4 for communication, and idempotent upsert operations for exactly-once semantics.

### **Prerequisites**

Begin by setting up a standard Maven or Gradle project. The following dependencies are essential and must be added to your project's pom.xml or build.gradle file:

* **org.apache.flink:flink-connector-base**: Provides the AsyncSinkBase and related classes.39  
* **com.azure:azure-cosmos**: The Azure Cosmos DB Java SDK v4, which includes the asynchronous client and bulk operation support.37  
* **org.slf4j:slf4j-api** and a logging implementation (e.g., **org.slf4j:slf4j-simple** or **ch.qos.logback:logback-classic**): For robust logging within the sink.

Ensure your project is configured to use Java 11 or newer, as this is a prerequisite for modern Flink versions and the Cosmos DB SDK.58

### **Building the Sink with AsyncSinkBase**

The implementation is structured into three primary classes that work together: an ElementConverter to prepare requests, an AsyncSinkWriter to execute them, and the main Sink class to tie everything together.

#### **Step 1: The ElementConverter**

The ElementConverter is a simple but crucial component that decouples the sink's internal logic from the specific data type (InputT) flowing in the Flink DataStream. Its sole purpose is to transform an incoming InputT object into a serializable RequestEntryT object. This RequestEntryT serves as an internal representation that contains all the information needed to perform a write operation.

For our Cosmos DB sink, the RequestEntryT should be a simple POJO (Plain Old Java Object) containing the document to be written (as another POJO or a JsonNode) and the value of its partition key.

Java

// A simple POJO to hold the data for a single write operation.  
// It must be serializable as Flink may write it to checkpointed state.  
public class CosmosDBRequestEntry implements Serializable {  
    private final MyDocumentPojo document;  
    private final String partitionKey;

    public CosmosDBRequestEntry(MyDocumentPojo document, String partitionKey) {  
        this.document \= document;  
        this.partitionKey \= partitionKey;  
    }

    // Getters...  
}

// The ElementConverter implementation.  
public class MyCosmosDBElementConverter   
    implements ElementConverter\<MyDocumentPojo, CosmosDBRequestEntry\> {

    @Override  
    public CosmosDBRequestEntry apply(MyDocumentPojo element, SinkWriter.Context context) {  
        // The logic to extract the partition key from the document.  
        String partitionKeyValue \= element.getPartitioningField();  
        return new CosmosDBRequestEntry(element, partitionKeyValue);  
    }  
}

#### **Step 2: The AsyncSinkWriter**

The AsyncSinkWriter is the heart of the sink, containing the logic for batching, writing to Cosmos DB, and handling responses. It extends Flink's AsyncSinkWriter and implements its abstract methods.

Java

import com.azure.cosmos.models.CosmosBulkOperations;  
import com.azure.cosmos.models.CosmosItemOperation;  
import com.azure.cosmos.models.PartitionKey;  
import org.apache.flink.api.connector.sink2.SinkWriter;  
//... other imports

public class MyCosmosDBSinkWriter   
    extends AsyncSinkWriter\<MyDocumentPojo, CosmosDBRequestEntry\> {

    private final transient CosmosAsyncContainer container;  
    private final transient ObjectMapper objectMapper \= new ObjectMapper();

    public MyCosmosDBSinkWriter(  
        ElementConverter\<MyDocumentPojo, CosmosDBRequestEntry\> elementConverter,  
        Sink.InitContext context,  
        CosmosAsyncContainer container  
    ) {  
        super(elementConverter, context, new AsyncSinkWriter.WriterConfiguration());  
        this.container \= container;  
    }

    @Override  
    protected void submitRequestEntries(  
        List\<CosmosDBRequestEntry\> requestEntries,  
        Consumer\<List\<CosmosDBRequestEntry\>\> requestResult  
    ) {  
        // Group entries by partition key to leverage Cosmos DB bulk operations efficiently.  
        Map\<String, List\<CosmosDBRequestEntry\>\> groupedByPartitionKey \=  
            requestEntries.stream().collect(Collectors.groupingBy(CosmosDBRequestEntry::getPartitionKey));

        List\<CosmosDBRequestEntry\> failedEntries \= new CopyOnWriteArrayList\<\>();  
        CountDownLatch latch \= new CountDownLatch(groupedByPartitionKey.size());

        for (Map.Entry\<String, List\<CosmosDBRequestEntry\>\> entry : groupedByPartitionKey.entrySet()) {  
            List\<CosmosItemOperation\> operations \= entry.getValue().stream()  
              .map(req \-\> CosmosBulkOperations.getUpsertItemOperation(  
                    req.getDocument(), new PartitionKey(req.getPartitionKey())))  
              .collect(Collectors.toList());

            container.executeBulkOperations(Flux.fromIterable(operations))  
              .doOnComplete(() \-\> latch.countDown())  
              .doOnError(err \-\> {  
                    // If the entire bulk operation fails, retry all entries in this batch.  
                    failedEntries.addAll(entry.getValue());  
                    latch.countDown();  
                })  
              .subscribe(response \-\> {  
                    if (\!response.getResponse().isSuccessStatusCode()) {  
                        // If a specific operation within the batch fails, retry it.  
                        // A more robust implementation would check the status code for retryability (e.g., 429, 449).  
                        CosmosDBRequestEntry originalEntry \= findOriginalEntry(response.getOperation(), entry.getValue());  
                        if (originalEntry\!= null) {  
                            failedEntries.add(originalEntry);  
                        }  
                    }  
                });  
        }

        try {  
            // Wait for all async bulk operations to complete.  
            latch.await();  
        } catch (InterruptedException e) {  
            Thread.currentThread().interrupt();  
            // On interruption, assume all requests failed and retry them.  
            failedEntries.addAll(requestEntries);  
        }  
          
        requestResult.accept(failedEntries);  
    }

    @Override  
    protected long getSizeInBytes(CosmosDBRequestEntry requestEntry) {  
        // Provide an estimate of the entry's size for Flink's size-based buffering.  
        try {  
            return objectMapper.writeValueAsBytes(requestEntry.getDocument()).length;  
        } catch (JsonProcessingException e) {  
            return 1024; // Fallback size  
        }  
    }  
      
    // Helper method to map a failed operation back to its original RequestEntryT  
    private CosmosDBRequestEntry findOriginalEntry(...) { /\*... \*/ }  
}

The core logic resides in submitRequestEntries. This method receives a list of buffered entries from the AsyncSinkBase framework. The implementation groups these entries by their partition key, as CosmosBulkOperations are most efficient when operating on data within the same logical partition. It then creates and executes an asynchronous bulk upsert operation for each group. The reactive stream's response is handled to identify any failed operations, which are then passed back to the AsyncSinkBase framework via the requestResult consumer for a potential retry.

#### **Step 3: The Sink Class**

The final piece is the main sink class, which extends AsyncSinkBase. Its primary role is to act as a factory for the AsyncSinkWriter and to manage the lifecycle of the CosmosAsyncClient.

Java

import org.apache.flink.api.connector.sink2.Sink;  
import org.apache.flink.api.connector.sink2.StatefulSink;  
//... other imports

public class MyCosmosDBSink   
    extends AsyncSinkBase\<MyDocumentPojo, CosmosDBRequestEntry\> {

    private final MyCosmosDBConfig config;

    public MyCosmosDBSink(MyCosmosDBConfig config) {  
        super(new MyCosmosDBElementConverter(),   
              config.getMaxBatchSize(),   
              config.getMaxInFlightRequests(),  
              config.getMaxBufferedRequests(),  
              config.getMaxBatchSizeInBytes(),  
              config.getMaxTimeInBufferMS(),  
              config.getMaxRecordSizeInBytes());  
        this.config \= config;  
    }

    @Override  
    public StatefulSinkWriter\<MyDocumentPojo, BufferedRequestState\<CosmosDBRequestEntry\>\> createWriter(InitContext context) {  
        CosmosAsyncClient client \= new CosmosClientBuilder()  
          .endpoint(config.getEndpoint())  
          .key(config.getKey())  
          .consistencyLevel(ConsistencyLevel.SESSION)  
          .contentResponseOnWriteEnabled(true)  
          .buildAsyncClient();  
              
        CosmosAsyncContainer container \= client  
          .getDatabase(config.getDatabase())  
          .getContainer(config.getContainer());

        return new MyCosmosDBSinkWriter(  
            getElementConverter(),  
            context,  
            container  
        );  
    }  
}

This class takes a configuration object (MyCosmosDBConfig) containing connection details and performance parameters. In the createWriter method, it instantiates the CosmosAsyncClient and retrieves a reference to the target container, which are then passed to the MyCosmosDBSinkWriter.

### **Handling Backpressure and Throttling**

The interaction between Flink's backpressure and Cosmos DB's throttling is a critical aspect of the sink's performance and stability.

The Azure Cosmos DB SDK has a built-in, configurable retry policy for handling HTTP 429 (throttling) responses. The ThrottlingRetryOptions allow you to set the maximum number of retries and the total maximum wait time. This mechanism is effective for handling short, intermittent bursts of throttling.

However, in cases of sustained overload where Cosmos DB consistently returns 429s, relying solely on the SDK's retry logic can be detrimental. The SDK threads will block, waiting for the retry-after duration, which can lead to timeouts within the Flink operator itself.

This is where the AsyncSinkBase's backpressure mechanism, configured by max-in-flight-requests, becomes essential. This parameter limits the number of concurrent asynchronous requests the sink will issue. When this limit is reached, the AsyncSinkWriter will not accept any more records from Flink until one of the in-flight requests completes. This stop-and-wait signal naturally propagates up the Flink job graph, causing upstream operators and eventually the source to slow down, thus reducing the load on the sink and allowing Cosmos DB to recover.

The optimal strategy is a two-layered approach:

1. **Cosmos SDK:** Configure a conservative retry policy (e.g., 2-3 retries with a short max wait time) to handle micro-bursts without introducing excessive latency.  
2. **Flink AsyncSinkBase:** Configure max-in-flight-requests to a reasonable value (e.g., 5-10, depending on parallelism and expected latency) to manage sustained backpressure at the stream processing level.

### **Custom Sink Configuration Parameters**

A production-ready sink requires a comprehensive set of configurable parameters to allow for tuning in different environments.

| Parameter Name | Scope | Description | Default Value |
| :---- | :---- | :---- | :---- |
| cosmos.endpoint | Cosmos SDK | The URI of the Azure Cosmos DB account. | N/A (Required) |
| cosmos.key | Cosmos SDK | The primary or secondary key for the Cosmos DB account. | N/A (Required) |
| cosmos.database | Cosmos SDK | The name of the target database. | N/A (Required) |
| cosmos.container | Cosmos SDK | The name of the target container. | N\_A (Required) |
| sink.buffer-flush.max-rows | Flink | The maximum number of records to buffer before flushing. | 1000 |
| sink.buffer-flush.max-size-in-bytes | Flink | The maximum size of buffered records in bytes before flushing. | 5 MB |
| sink.buffer-flush.interval | Flink | The maximum time interval (in ms) between flushes. | 1000 |
| sink.max-in-flight-requests | Flink | The maximum number of concurrent asynchronous requests. | 50 |
| cosmos.maxRetryAttemptsOnThrottledRequests | Cosmos SDK | Max retry attempts in the SDK on HTTP 429 responses. | 9 |
| cosmos.maxRetryWaitTime | Cosmos SDK | Max total time the SDK will spend retrying throttled requests. | 30s |

## **Implementation in Python (PyFlink): Bridging the JVM Divide**

Implementing a custom sink for a PyFlink application presents a unique architectural challenge. Because Apache Flink's core runtime is built on the Java Virtual Machine (JVM), and PyFlink acts as a Python API wrapper that communicates with this JVM runtime, low-level components like sources and sinks cannot be implemented in pure Python.60 Instead, they must be developed as standard Java (or Scala) components and then made accessible to the PyFlink environment.45 This approach leverages the performance and maturity of the JVM ecosystem for heavy I/O operations while providing the convenience and familiarity of Python for defining the overall dataflow logic.

### **Architectural Approach: The Java Core**

The task of "creating a PyFlink sink for Cosmos DB" is, in practice, the task of creating a robust Java sink (as detailed in the previous section) and then invoking it from a PyFlink application. The Python code does not contain the sink's operational logic; rather, it declaratively defines the sink and directs the Flink runtime to use the provided Java implementation.45 This approach leverages the performance and maturity of the JVM ecosystem for heavy I/O operations while providing the convenience and familiarity of Python for defining the overall dataflow logic.

### **Packaging the Java Sink for PyFlink**

The first step is to package the Java sink, developed in Section IV, into a self-contained "fat JAR" or "uber JAR." This is crucial because the JAR must be submitted to the Flink cluster along with the Python script, and it must contain not only the custom sink code but also all of its transitive dependencies, including the Azure Cosmos DB SDK and its dependencies.

This can be achieved using standard build tools:

* **Maven:** Use the maven-shade-plugin. Configure it to bundle all dependencies into a single JAR.  
* **Gradle:** Use the shadowJar plugin, which serves the same purpose.

The build process will produce a single JAR file (e.g., flink-cosmosdb-sink-1.0-all.jar) that can be easily distributed and added to the Flink classpath.

### **Using the Custom Sink in PyFlink Table API / SQL**

The Flink Table API and SQL provide the most straightforward way to use a custom Java connector from Python. The integration is handled declaratively through Flink's connector discovery mechanism.

1. **Connector Factory (Java):** To make the sink discoverable by the Table API, the Java project must include a factory class that implements DynamicTableSinkFactory. This factory is responsible for parsing the options from the DDL WITH clause and creating an instance of the custom sink. The factory is registered as a Java Service Provider by creating a file in src/main/resources/META-INF/services/org.apache.flink.table.factories.Factory that contains the fully qualified name of the factory class. The factory's factoryIdentifier() method defines the string that will be used in the SQL DDL (e.g., 'my-cosmosdb').  
2. **Configuration (Python):** The PyFlink script must be configured to include the custom JAR in its classpath. This informs the Flink cluster where to find the necessary Java classes for the custom connector. This can be done in several ways:  
   * **Command Line:** Pass the JAR path using the \--jarfile argument when submitting the job: flink run \--jarfile /path/to/flink-cosmosdb-sink-1.0-all.jar \-py my\_app.py.  
   * **In-Code Configuration:** Set the pipeline.jars configuration property on the TableEnvironment. This is often the more maintainable approach as it keeps the dependency declaration with the code.45

Python  
from pyflink.table import EnvironmentSettings, TableEnvironment

env\_settings \= EnvironmentSettings.in\_streaming\_mode()  
t\_env \= TableEnvironment.create(env\_settings)

\# Add the custom sink JAR to the classpath  
\# The path must be accessible by the Flink cluster nodes.  
t\_env.get\_config().set(  
    "pipeline.jars", "file:///path/to/flink-cosmosdb-sink-1.0-all.jar"  
)

3. **Defining the Sink Table (DDL):** With the JAR available, you can define a sink table using a standard CREATE TABLE DDL statement. The WITH clause is used to pass all necessary configuration parameters to the Java DynamicTableSinkFactory.

### **Code Example**

The following Python script demonstrates the complete workflow: setting up the environment, adding the custom JAR, defining a source and the custom Cosmos DB sink, and executing an INSERT statement to move data between them.

Python

import os  
from pyflink.table import EnvironmentSettings, TableEnvironment

def main():  
    \# 1\. Set up the PyFlink Table Environment  
    env\_settings \= EnvironmentSettings.in\_streaming\_mode()  
    t\_env \= TableEnvironment.create(env\_settings)

    \# 2\. Add the custom Java sink JAR to the Flink job's classpath  
    \# Assumes the JAR is placed in a 'jars' directory accessible by Flink.  
    jar\_path \= "file://" \+ os.path.abspath("jars/flink-cosmosdb-sink-1.0-all.jar")  
    t\_env.get\_config().set("pipeline.jars", jar\_path)  
      
    \# Retrieve credentials securely from environment variables  
    cosmos\_endpoint \= os.environ.get("COSMOS\_ENDPOINT")  
    cosmos\_key \= os.environ.get("COSMOS\_KEY")

    \# 3\. Define a source table (e.g., a datagen source for demonstration)  
    source\_ddl \= """  
        CREATE TABLE datagen\_source (  
            id STRING,  
            product\_name STRING,  
            category STRING,  
            price DOUBLE  
        ) WITH (  
            'connector' \= 'datagen',  
            'rows-per-second' \= '10',  
            'fields.id.kind' \= 'sequence',  
            'fields.id.start' \= '1',  
            'fields.id.end' \= '10000',  
            'fields.product\_name.length' \= '10',  
            'fields.category.length' \= '5',  
            'fields.price.min' \= '10',  
            'fields.price.max' \= '1000'  
        )  
    """  
    t\_env.execute\_sql(source\_ddl)

    \# 4\. Define the sink table using the custom Cosmos DB connector  
    sink\_ddl \= f"""  
        CREATE TABLE cosmosdb\_sink (  
            id STRING,  
            product\_name STRING,  
            category STRING,  
            price DOUBLE,  
            PRIMARY KEY (id) NOT ENFORCED  
        ) WITH (  
            'connector' \= 'my-cosmosdb',  
            'cosmos.endpoint' \= '{cosmos\_endpoint}',  
            'cosmos.key' \= '{cosmos\_key}',  
            'cosmos.database' \= 'flink-database',  
            'cosmos.container' \= 'products',  
            'sink.buffer-flush.max-rows' \= '500'  
        )  
    """  
    t\_env.execute\_sql(sink\_ddl)

    \# 5\. Define the data processing logic and execute the insert  
    t\_env.from\_path("datagen\_source") \\  
      .execute\_insert("cosmosdb\_sink") \\  
      .wait()

if \_\_name\_\_ \== "\_\_main\_\_":  
    main()

In this example, the 'connector' \= 'my-cosmosdb' line instructs Flink's Table API to find a factory with the identifier "my-cosmosdb". The parameters in the WITH clause are then passed as a map to the factory, which uses them to configure and instantiate the Java sink implementation.

### **Limitations and Considerations**

* **Performance:** While the data writing itself is performed efficiently on the JVM by the Java sink, any preceding transformations that use Python User-Defined Functions (UDFs) will incur a serialization/deserialization overhead as data moves between the Python UDF worker and the Flink JVM. For performance-critical paths, it is advisable to implement transformations in Java or using Flink SQL's built-in functions where possible.  
* **Debugging:** Troubleshooting becomes a cross-language endeavor. Errors can originate in the Python script (e.g., incorrect DDL), in the Flink JVM runtime (e.g., class loading issues), or within the Java sink's logic (e.g., authentication failure with Cosmos DB). Effective debugging requires analyzing logs from both the Python client and the Flink TaskManager JVMs.  
* **Data Serialization:** The PyFlink Table API transparently handles the conversion of Python data types into Flink's internal RowData format, which is then passed to the Java sink. While this is seamless for standard types, it's an important underlying mechanism to be aware of, especially when dealing with complex or nested data structures.

## **Implementation in Scala: A JVM-Native Alternative**

Implementing the custom Cosmos DB sink in Scala offers a JVM-native alternative to Java that many developers find more concise and expressive. The fundamental architectural approach remains identical to the Java implementation, as Scala applications compile to JVM bytecode and have seamless interoperability with Java libraries. This means the same Flink APIs (AsyncSinkBase) and the same Azure Cosmos DB Java SDK v4 are the core components of the solution.

### **Approach: Leveraging Java Interoperability**

The core logic of the sink—connecting to Cosmos DB, converting elements, batching requests, and handling responses—will mirror the Java reference implementation. Scala's primary advantage lies in its ability to express this logic with less boilerplate code and leverage powerful language features.

* **Data Models:** Scala's case class provides an immutable and concise way to define the data models for both the incoming Flink records (InputT) and the internal request entries (RequestEntryT). They come with equals, hashCode, toString, and other methods automatically generated, reducing boilerplate.  
* **Java Library Usage:** The Azure Cosmos DB Java SDK v4 can be used directly from Scala. Scala's interoperability allows for instantiating and calling methods on Java classes (like CosmosAsyncClientBuilder and CosmosAsyncContainer) as if they were native Scala classes. The reactive types from Project Reactor (Flux, Mono) used by the SDK can also be handled in Scala, either directly or by converting them to Scala's native Future or a functional effects library like ZIO or Cats-Effect for more idiomatic asynchronous programming.  
* **Flink Scala API:** While the core sink implementation will use the Java-based AsyncSinkBase, the surrounding Flink application can leverage the flink-scala-api wrapper.66 This provides more idiomatic Scala entry points for creating the  
  StreamExecutionEnvironment and working with DataStream objects, along with improved type information and serialization for Scala types like case classes.

### **Key Syntactic and Idiomatic Differences**

While the design pattern is the same, the Scala implementation will look distinct.

* **Class and Trait Definitions:** Scala's class and trait syntax is used instead of Java's. The ElementConverter can be implemented as a simple class, while the SinkWriter and Sink will extend the respective Flink Java base classes.  
* **Functional Collections API:** Scala's rich and powerful collections API can be used to simplify data transformations, such as the grouping of request entries by partition key.  
* **Concurrency:** While direct use of Project Reactor's Flux is possible, a common pattern in Scala is to convert the reactive stream to a scala.concurrent.Future. This can be done using a helper library or a simple bridge. For example, a CompletableFuture (which can be obtained from a Mono) is easily converted to a Scala Future.

### **Code Example**

The following code provides a sketch of the Scala implementation, highlighting the syntactic differences and idiomatic usage compared to the Java version.

Scala

import com.azure.cosmos.models.{CosmosBulkOperations, PartitionKey}  
import com.azure.cosmos.CosmosAsyncContainer  
import org.apache.flink.api.connector.sink2.{AsyncSinkBase, ElementConverter, Sink, StatefulSinkWriter}  
import java.util.function.Consumer  
import scala.collection.JavaConverters.\_  
import scala.concurrent.{Await, Future, Promise}  
import scala.concurrent.ExecutionContext.Implicits.global

// Define the data model and request entry using case classes  
case class MyDocument(id: String, category: String, data: String)  
case class CosmosDBRequest(document: MyDocument, partitionKey: String)

// ElementConverter implementation in Scala  
class MyCosmosDBElementConverter extends ElementConverter {  
  override def apply(element: MyDocument, context: Sink.Writer.Context): CosmosDBRequest \= {  
    CosmosDBRequest(element, element.category)  
  }  
}

// AsyncSinkWriter implementation in Scala  
class MyCosmosDBSinkWriter(  
  elementConverter: ElementConverter,  
  context: Sink.InitContext,  
  container: CosmosAsyncContainer  
) extends AsyncSinkWriter(elementConverter, context, new AsyncSinkWriter.WriterConfiguration()) {

  override def submitRequestEntries(  
    requestEntries: java.util.List,  
    requestResult: Consumer\]  
  ): Unit \= {  
    val failedEntries \= new java.util.concurrent.CopyOnWriteArrayList()

    val futures \= requestEntries.asScala  
    .groupBy(\_.partitionKey)  
    .map { case (pk, entries) \=\>  
        val operations \= entries.map(req \=\>  
          CosmosBulkOperations.getUpsertItemOperation(req.document, new PartitionKey(pk))  
        ).asJava

        // Bridge from Reactor Mono to Scala Future  
        val promise \= Promise\[Unit\]()  
        container.executeBulkOperations(Flux.fromIterable(operations))  
        .doOnComplete(() \=\> promise.success(()))  
        .doOnError(err \=\> {  
            failedEntries.addAll(entries.asJava)  
            promise.failure(err)  
          })  
        .subscribe(response \=\> {  
            if (\!response.getResponse.isSuccessStatusCode) {  
              // Find original entry and add to failedEntries  
            }  
          })  
        promise.future  
      }

    // Await completion of all futures  
    val allFutures \= Future.sequence(futures)  
    try {  
      Await.result(allFutures, scala.concurrent.duration.Duration.Inf)  
    } catch {  
      case e: Exception \=\> // Handle timeout or other exceptions  
        failedEntries.addAll(requestEntries)  
    }  
      
    requestResult.accept(failedEntries)  
  }  
    
  override def getSizeInBytes(requestEntry: CosmosDBRequest): Long \= {  
    // Estimate size, similar to Java implementation  
    requestEntry.toString.getBytes.length.toLong  
  }  
}

// Main Sink class in Scala  
class MyCosmosDBSink(config: MyCosmosDBConfig)   
  extends AsyncSinkBase(  
    new MyCosmosDBElementConverter(),  
    // Pass configuration parameters from config object...  
  ) {  
    
  override def createWriter(context: Sink.InitContext): StatefulSinkWriter\] \= {  
    val client \= new CosmosClientBuilder()  
      //... configure client from config object  
    .buildAsyncClient()  
        
    val container \= client.getDatabase(config.database).getContainer(config.container)

    new MyCosmosDBSinkWriter(getElementConverter, context, container)  
  }  
}

This Scala example demonstrates that while the underlying Flink and Cosmos DB APIs are Java-based, Scala provides a more functional and concise syntax for their implementation. The core design principles—batching by partition key, asynchronous execution, and handling failures for retry—remain unchanged. For teams proficient in Scala, this approach offers a highly effective and maintainable way to build a custom Flink sink.

## **Advanced Topics: Performance Tuning, Cost Optimization, and Operations**

Developing a functional sink is only the first step. A production-ready connector must be tunable, cost-effective, and observable. This section covers the advanced topics essential for operating the custom Flink-to-Cosmos DB sink successfully in a production environment, focusing on the interplay between Flink's performance characteristics and Cosmos DB's consumption-based pricing model.

### **Optimizing Cosmos DB Throughput (RU/s)**

The cost and performance of Azure Cosmos DB are primarily governed by Request Units per second (RU/s). Every database operation, from reads and writes to queries, consumes RUs. A primary goal of the custom sink is to write data as efficiently as possible, minimizing the RU cost per record.

* **Partition Key Strategy:** This is the single most critical factor for achieving performance and scalability in Cosmos DB. The partition key determines how data is distributed across physical partitions. For a write-heavy workload like a Flink sink, it is imperative to choose a partition key with high cardinality (a large number of unique values). This ensures that incoming writes from the parallel Flink sink tasks are spread evenly across all of Cosmos DB's physical partitions, preventing "hot partitions" where a single partition receives a disproportionate amount of traffic and becomes a bottleneck. The sink implementation naturally groups operations by partition key for its bulk requests; a good key design ensures these groups are numerous and small, rather than few and large.  
* **Indexing Policy:** By default, Azure Cosmos DB indexes every property of every document written to a container. While this provides maximum query flexibility, it incurs a significant RU cost on every write operation, as each indexed property must be updated. For write-intensive applications, the default policy is often suboptimal. It is a critical best practice to customize the indexing policy to include only the properties that are actively used as filters in queries. Excluding unnecessary properties from the index can dramatically reduce the RU charge for writes, thereby increasing the number of records that can be ingested per second for the same provisioned throughput.  
* **Item Sizing and Data Modeling:** The RU cost of an operation is directly correlated with the size of the document. Storing very large items (e.g., multi-megabyte JSON documents) is an anti-pattern that leads to high RU charges and can strain the system. A best practice is to right-size documents, storing only the necessary operational data. For large binary content or extensive text that does not need to be queried, it is more cost-effective to store this data in a service like Azure Blob Storage and include only a reference or URL to it in the Cosmos DB item.  
* **Consistency Levels:** Azure Cosmos DB offers five distinct consistency levels, each with different performance and cost trade-offs. Stronger consistency levels, such as "Strong," consume approximately twice the RUs for writes compared to weaker levels like "Session," "Consistent Prefix," and "Eventual," because they must synchronously commit the write to a quorum of replicas. For many Flink streaming use cases where data is processed in-order within a partition, "Session" consistency provides an excellent balance of strong consistency for a given client session and lower RU cost.

### **Tuning Flink for the Sink**

The performance of the sink is also heavily dependent on the configuration of the Flink job itself.

* **Parallelism:** The parallelism of the sink operator should be tuned based on the expected workload and the provisioned throughput of the Cosmos DB container. A higher parallelism allows Flink to send more concurrent requests to Cosmos DB, which can increase overall throughput. However, setting the parallelism too high can overwhelm the database, leading to excessive throttling. A good starting point is to align the sink parallelism with the number of partitions in the upstream Kafka topic or to scale it based on the number of physical partitions in the Cosmos DB container.  
* **Checkpointing Configuration:** The checkpointing interval (execution.checkpointing.interval) directly impacts fault tolerance and recovery time. A frequent checkpoint interval (e.g., every few seconds) minimizes the amount of data that needs to be replayed upon failure but introduces higher overhead from the checkpointing process itself.67 An infrequent interval (e.g., every few minutes) reduces this overhead but increases the recovery time and potential for data replay. For sinks providing exactly-once semantics via two-phase commit, the checkpoint interval also dictates the minimum end-to-end latency, as data is only made visible upon successful checkpoint completion..4  
* **Monitoring Backpressure:** The Flink Web UI is an indispensable tool for diagnosing performance bottlenecks. The "Backpressure" tab provides a visual representation of how busy each operator is and whether it is being slowed down by downstream operators. For the custom sink, a healthy state is one where the sink operator has a high "Busy" metric (indicating it is actively processing data) but a low "Backpressured" metric. If an upstream operator shows high "Backpressured" status (indicated by black coloring) and the sink operator is consistently "Busy" (red), it is a clear sign that the sink is the bottleneck in the pipeline.33 This indicates that either Flink is sending data faster than Cosmos DB can ingest it, and the sink's backpressure mechanism is working correctly, or there is an inefficiency in the sink's implementation.

### **Monitoring and Diagnostics**

Effective monitoring is crucial for maintaining the health and performance of the data pipeline. This requires visibility into both the Flink application and the Cosmos DB service.

* **Flink Metrics:** The custom sink implementation should expose key metrics using Flink's metrics system. The AsyncSinkBase already provides several useful metrics out of the box, including:  
  * numRecordsOut: The total number of records sent from the sink.  
  * numBytesOut: The total number of bytes sent from the sink.  
  * currentSendTime: A latency gauge for the sink's write operations.  
    Custom metrics can also be added to track things like the number of failed requests, the number of retries, and the batch size distribution.  
* **Cosmos DB Diagnostics:** The Azure Cosmos DB Java SDK v4 provides a powerful diagnostic feature. Every response object (e.g., CosmosBulkOperationResponse, CosmosItemResponse) contains a getDiagnostics() method that returns a detailed JSON string. This diagnostic information is invaluable for troubleshooting performance issues. It includes:  
  * Precise, multi-stage latency breakdowns for the request.  
  * The request and response sizes.  
  * The session token used.  
  * The specific Cosmos DB gateway and replica that served the request.  
  * Details of any retries that occurred, either on the client or server side.  
    The custom sink should be configured to log these diagnostics, especially for failed or slow requests, to provide deep insights into the interaction between the Flink client and the Cosmos DB service.

## **Conclusion and Strategic Recommendations**

The integration of Apache Flink's real-time, stateful processing capabilities with Azure Cosmos DB's globally distributed, scalable NoSQL database creates a powerful architecture for modern, event-driven applications. This report has provided a comprehensive analysis of the two primary architectural pathways for this integration and detailed the engineering principles required to build a custom, high-performance data sink that respects and leverages Flink's core state management features. The decision of which path to follow and how to implement it is critical and has long-term implications for a project's performance, cost, and maintainability.

### **Summary of Findings**

The investigation into connecting Flink and Cosmos DB has yielded several key conclusions. Firstly, a fundamental architectural choice exists between an indirect path, which uses Apache Kafka and the Kafka Connect framework as an intermediary, and a direct path, which involves developing a custom Flink sink. The indirect path offers the benefits of operational decoupling and a mature, configuration-driven connector, but at the cost of increased latency and infrastructure complexity. The direct path provides minimal latency and maximum control but demands significant custom development and maintenance effort.

For the direct path, the optimal implementation strategy is to build upon Flink's modern AsyncSinkBase API in conjunction with the Azure Cosmos DB Java SDK v4. This combination abstracts away the most complex aspects of sink development—such as buffering, stateful checkpointing, and asynchronous request management—while providing the tools necessary for high-throughput bulk writing.

Furthermore, achieving end-to-end exactly-once semantics, a crucial guarantee for data integrity in stateful applications, is best accomplished not through a complex two-phase commit protocol, but through a more pragmatic and idiomatic pattern of idempotent writes. By combining the at-least-once delivery guarantee of AsyncSinkBase with the idempotent nature of Cosmos DB's upsert operation, a system can ensure a correct and consistent final state even in the face of failures and data replay.

Finally, while the reference implementation is best developed in Java, this core component can be seamlessly utilized by applications written in Python (PyFlink) and Scala. This is due to PyFlink's reliance on a JVM runtime for low-level connectors and Scala's excellent interoperability with Java libraries.

### **Final Recommendations**

Based on this comprehensive analysis, the following strategic recommendations are provided to guide teams in their implementation choices:

1. **Choose the Architectural Path Based on Core Priorities:**  
   * **Select the indirect path via Kafka Connect** if your organization already has a mature Kafka ecosystem, if the development team's primary strength is in configuration and operations rather than custom Java development, or if the application can tolerate the moderate increase in end-to-end latency. This path prioritizes time-to-market and leverages a supported, feature-rich component.  
   * **Commit to building a direct custom sink** when low latency is a non-negotiable requirement, when the write logic is complex (e.g., requiring dynamic routing based on record content), or when the architectural goal is to minimize the number of independent systems in the data path. This path prioritizes performance and control.  
2. **For Custom Sinks, Adopt the Recommended Technology Stack:**  
   * **Build the sink in Java** as the primary implementation language. This provides the broadest access to community knowledge, examples, and direct support for Flink's core APIs and the Azure SDK.  
   * Use **Flink's AsyncSinkBase** as the foundational framework. Do not attempt to re-implement its complex buffering, checkpointing, and backpressure logic from scratch.  
   * Utilize the **Azure Cosmos DB Java SDK v4**, specifically the CosmosAsyncClient and its executeBulkOperations functionality, to achieve the highest possible write throughput.  
   * Implement **idempotent writes using upsertItem** as the primary strategy for achieving exactly-once semantics. Ensure that the data model includes a unique identifier suitable for use as the Cosmos DB id.  
3. **Treat the Sink as a Mission-Critical Component:**  
   * Invest heavily in **performance tuning and cost optimization**. Proactively design a high-cardinality partition key, customize the indexing policy to minimize write RUs, and right-size documents.  
   * Implement **comprehensive monitoring and logging**. Capture Flink's backpressure metrics and the detailed diagnostics from the Cosmos DB SDK to enable rapid troubleshooting of performance issues.  
   * Manage **credentials securely** using a dedicated secrets management system appropriate for your deployment environment (e.g., Kubernetes Secrets, Azure Key Vault).

By following these recommendations, engineering teams can confidently design and deploy a robust, scalable, and efficient data pipeline that effectively bridges the powerful stateful stream processing capabilities of Apache Flink with the global-scale persistence of Azure Cosmos DB.

#### **Works cited**

1. Use Cases \- Apache Flink, accessed August 29, 2025, [https://flink.apache.org/what-is-flink/use-cases/](https://flink.apache.org/what-is-flink/use-cases/)  
2. This One Apache Flink Pattern Changed Everything: Stateful Event-Driven Processing with KeyedProcessFunction | by CortexFlow | The Software Frontier | Medium, accessed August 29, 2025, [https://medium.com/the-software-frontier/this-one-apache-flink-pattern-changed-everything-stateful-event-driven-processing-with-1fd92f64fe79](https://medium.com/the-software-frontier/this-one-apache-flink-pattern-changed-everything-stateful-event-driven-processing-with-1fd92f64fe79)  
3. Stateful Stream Processing: Concepts, Tools, & Challenges \- Estuary, accessed August 29, 2025, [https://estuary.dev/blog/stateful-stream-processing/](https://estuary.dev/blog/stateful-stream-processing/)  
4. A simple guide to processing guarantees in Apache Flink | by Gyula Fóra \- Medium, accessed August 29, 2025, [https://medium.com/cloudera-inc/a-simple-guide-to-processing-guarantees-in-apache-flink-ca7e70431fdc](https://medium.com/cloudera-inc/a-simple-guide-to-processing-guarantees-in-apache-flink-ca7e70431fdc)  
5. Mastering Exactly-Once Processing in Apache Flink \- RisingWave, accessed August 29, 2025, [https://risingwave.com/blog/mastering-exactly-once-processing-in-apache-flink/](https://risingwave.com/blog/mastering-exactly-once-processing-in-apache-flink/)  
6. Flink Checkpoints Principles and Practices: Flink Advanced Tutorials \- Alibaba Cloud, accessed August 29, 2025, [https://www.alibabacloud.com/blog/flink-checkpoints-principles-and-practices-flink-advanced-tutorials\_596631](https://www.alibabacloud.com/blog/flink-checkpoints-principles-and-practices-flink-advanced-tutorials_596631)  
7. Kafka Connect Cosmos DB Sink Connector \- Microsoft Open Source, accessed August 29, 2025, [https://microsoft.github.io/kafka-connect-cosmosdb/doc/README\_Sink.html](https://microsoft.github.io/kafka-connect-cosmosdb/doc/README_Sink.html)  
8. Use Kafka Connect for Azure Cosmos DB to read and write data | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/kafka-connector](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/kafka-connector)  
9. Kafka Connect connectors for Azure Cosmos DB \- GitHub, accessed August 29, 2025, [https://github.com/microsoft/kafka-connect-cosmosdb](https://github.com/microsoft/kafka-connect-cosmosdb)  
10. Confluent Releases Managed V2 Connector for Apache Kafka® for Azure Cosmos DB, accessed August 29, 2025, [https://www.confluent.io/blog/event-streaming-cosmos-db/](https://www.confluent.io/blog/event-streaming-cosmos-db/)  
11. Kafka Connect for Azure Cosmos DB \- Sink Connector v2, accessed August 29, 2025, [https://docs.azure.cn/en-us/cosmos-db/nosql/kafka-connector-sink-v2](https://docs.azure.cn/en-us/cosmos-db/nosql/kafka-connector-sink-v2)  
12. Copy and transform data in Azure Cosmos DB for NoSQL by using Azure Data Factory, accessed August 29, 2025, [https://docs.azure.cn/en-us/data-factory/connector-azure-cosmos-db](https://docs.azure.cn/en-us/data-factory/connector-azure-cosmos-db)  
13. Tutorial: Connect to Azure Cosmos DB for NoSQL by using Spark, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/tutorial-spark-connector](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/tutorial-spark-connector)  
14. pagopa-archive/flink-poc: Exploring Apache Flink features and trying to perform operations on multiple streams coming from different sources. \- GitHub, accessed August 29, 2025, [https://github.com/pagopa/flink-poc](https://github.com/pagopa/flink-poc)  
15. apache/flink-cdc: Flink CDC is a streaming data integration tool \- GitHub, accessed August 29, 2025, [https://github.com/apache/flink-cdc](https://github.com/apache/flink-cdc)  
16. apache/flink-connector-aws \- GitHub, accessed August 29, 2025, [https://github.com/apache/flink-connector-aws](https://github.com/apache/flink-connector-aws)  
17. Deploy a Flink SQL Statement Using CI/CD and Confluent Cloud for Apache Flink, accessed August 29, 2025, [https://docs.confluent.io/cloud/current/flink/operate-and-deploy/deploy-flink-sql-statement.html](https://docs.confluent.io/cloud/current/flink/operate-and-deploy/deploy-flink-sql-statement.html)  
18. Azure/flink-connector-kusto \- GitHub, accessed August 29, 2025, [https://github.com/Azure/flink-connector-kusto](https://github.com/Azure/flink-connector-kusto)  
19. Bridging Flink SQL and Custom Java Pipelines with the Decodable SDK, accessed August 29, 2025, [https://www.decodable.co/blog/bridging-flink-sql-and-custom-java-pipelines-with-the-decodable-sdk](https://www.decodable.co/blog/bridging-flink-sql-and-custom-java-pipelines-with-the-decodable-sdk)  
20. Performance tips for Azure Cosmos DB Java SDK v4, accessed August 29, 2025, [https://docs.azure.cn/en-us/cosmos-db/nosql/performance-tips-java-sdk-v4](https://docs.azure.cn/en-us/cosmos-db/nosql/performance-tips-java-sdk-v4)  
21. Http Connector for Apache Flink. Provides sources and sinks for Datastream , Table and SQL APIs. \- GitHub, accessed August 29, 2025, [https://github.com/getindata/flink-http-connector](https://github.com/getindata/flink-http-connector)  
22. Building Real-Time Streaming Pipelines with Apache Flink & PyFlink | by Yousef Yousefi, accessed August 29, 2025, [https://usefusefi.medium.com/building-real-time-streaming-pipelines-with-apache-flink-pyflink-c8c2f26eebd8](https://usefusefi.medium.com/building-real-time-streaming-pipelines-with-apache-flink-pyflink-c8c2f26eebd8)  
23. Building Applications with Apache Flink (Part 4): Writing and Using a custom PostgreSQL SinkFunction \- bytefish.de, accessed August 29, 2025, [https://www.bytefish.de/blog/apache\_flink\_series\_4.html](https://www.bytefish.de/blog/apache_flink_series_4.html)  
24. Back pressure when using Flink FileSink \- amazon s3 \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/79424289/back-pressure-when-using-flink-filesink](https://stackoverflow.com/questions/79424289/back-pressure-when-using-flink-filesink)  
25. How Apache Flink handles backpressure? | by Kamal Maiti | Jul, 2025 \- Medium, accessed August 29, 2025, [https://medium.com/@kamal.maiti/how-apache-flink-handles-backpressure-e447eebca0c1](https://medium.com/@kamal.maiti/how-apache-flink-handles-backpressure-e447eebca0c1)  
26. Backpressure \- Managed Service for Apache Flink \- AWS Documentation, accessed August 29, 2025, [https://docs.aws.amazon.com/managed-flink/latest/java/troubleshooting-backpressure.html](https://docs.aws.amazon.com/managed-flink/latest/java/troubleshooting-backpressure.html)  
27. How to handle backpressure in flink streaming job? \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/45255575/how-to-handle-backpressure-in-flink-streaming-job](https://stackoverflow.com/questions/45255575/how-to-handle-backpressure-in-flink-streaming-job)  
28. Optimize your Azure Cosmos DB application using rate limiting | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/rate-limiting-requests](https://learn.microsoft.com/en-us/azure/cosmos-db/rate-limiting-requests)  
29. Can Flink replace Kafka Connect : r/apachekafka \- Reddit, accessed August 29, 2025, [https://www.reddit.com/r/apachekafka/comments/1901fs7/can\_flink\_replace\_kafka\_connect/](https://www.reddit.com/r/apachekafka/comments/1901fs7/can_flink_replace_kafka_connect/)  
30. Combining Confluent Kafka Connect with Apache Flink vs Spark? \- Google Groups, accessed August 29, 2025, [https://groups.google.com/g/confluent-platform/c/7\_Me\_7QDd2U](https://groups.google.com/g/confluent-platform/c/7_Me_7QDd2U)  
31. Flink vs. Kafka: A Quick Guide to Stream Processing Engines \- AnalytixLabs, accessed August 29, 2025, [https://www.analytixlabs.co.in/blog/flink-vs-kafka/](https://www.analytixlabs.co.in/blog/flink-vs-kafka/)  
32. Use bulk executor Java library in Azure Cosmos DB to perform bulk import and update operations | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/bulk-executor-java](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/bulk-executor-java)  
33. Bulk writing in Azure Cosmos DB Java SDK \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/77819486/bulk-writing-in-azure-cosmos-db-java-sdk](https://stackoverflow.com/questions/77819486/bulk-writing-in-azure-cosmos-db-java-sdk)  
34. Azure Cosmos DB Java SDK v4 \- Exploring the new Async API \- Microsoft Developer Blogs, accessed August 29, 2025, [https://devblogs.microsoft.com/cosmosdb/java-sdk-v4-async-vs-sync/](https://devblogs.microsoft.com/cosmosdb/java-sdk-v4-async-vs-sync/)  
35. Transactional Batch Operations in Azure Cosmos DB | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/transactional-batch](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/transactional-batch)  
36. Bulk and Transactional Batch Operations with the Azure Cosmos DB .NET \- C\# Corner, accessed August 29, 2025, [https://www.c-sharpcorner.com/article/bulk-and-transactional-batch-operations-with-the-azure-cosmos-db-net/](https://www.c-sharpcorner.com/article/bulk-and-transactional-batch-operations-with-the-azure-cosmos-db-net/)  
37. Azure Cosmos DB Java SDK v4 for API for NoSQL release notes and resources, accessed August 29, 2025, [https://docs.azure.cn/en-us/cosmos-db/nosql/sdk-java-v4](https://docs.azure.cn/en-us/cosmos-db/nosql/sdk-java-v4)  
38. Azure Cosmos DB Client Library for Java | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/java/api/overview/azure/cosmos-readme?view=azure-java-stable](https://learn.microsoft.com/en-us/java/api/overview/azure/cosmos-readme?view=azure-java-stable)  
39. The Generic Asynchronous Base Sink \- Apache Flink, accessed August 29, 2025, [https://flink.apache.org/2022/03/16/the-generic-asynchronous-base-sink/](https://flink.apache.org/2022/03/16/the-generic-asynchronous-base-sink/)  
40. flink/flink-connectors/flink-connector-base/src/main/java/org/apache/flink/connector/base/sink/writer/AsyncSinkWriter.java at master \- GitHub, accessed August 29, 2025, [https://github.com/apache/flink/blob/master/flink-connectors/flink-connector-base/src/main/java/org/apache/flink/connector/base/sink/writer/AsyncSinkWriter.java](https://github.com/apache/flink/blob/master/flink-connectors/flink-connector-base/src/main/java/org/apache/flink/connector/base/sink/writer/AsyncSinkWriter.java)  
41. An Overview of End-to-End Exactly-Once Processing in Apache Flink (with Apache Kafka, too\!), accessed August 29, 2025, [https://flink.apache.org/2018/02/28/an-overview-of-end-to-end-exactly-once-processing-in-apache-flink-with-apache-kafka-too/](https://flink.apache.org/2018/02/28/an-overview-of-end-to-end-exactly-once-processing-in-apache-flink-with-apache-kafka-too/)  
42. Analytics don't want duplicated data, so get it exactly-once with Flink/Kafka, accessed August 29, 2025, [https://dev.to/kination/analytics-dont-want-duplicated-data-so-get-it-exactly-once-with-flinkkafka-ga4](https://dev.to/kination/analytics-dont-want-duplicated-data-so-get-it-exactly-once-with-flinkkafka-ga4)  
43. Extending Flink functions \- Agile Lab Engineering \- Medium, accessed August 29, 2025, [https://medium.com/agile-lab-engineering/extending-flink-functions-7976df8001e2](https://medium.com/agile-lab-engineering/extending-flink-functions-7976df8001e2)  
44. How we (almost :)) achieve End-to-End Exactly-Once processing with Flink | by Devora Roth Goldshmidt | CodeX | Medium, accessed August 29, 2025, [https://medium.com/codex/how-we-almost-achieve-end-to-end-exactly-once-processing-with-flink-28d2c013b5c1](https://medium.com/codex/how-we-almost-achieve-end-to-end-exactly-once-processing-with-flink-28d2c013b5c1)  
45. A Hands-On Introduction to PyFlink \- Decodable, accessed August 29, 2025, [https://www.decodable.co/blog/a-hands-on-introduction-to-pyflink](https://www.decodable.co/blog/a-hands-on-introduction-to-pyflink)  
46. Exactly-Once Processing in Apache Flink \- Confluent Developer, accessed August 29, 2025, [https://developer.confluent.io/learn/streamables/exactly-once-processing-in-apache-flink/](https://developer.confluent.io/learn/streamables/exactly-once-processing-in-apache-flink/)  
47. Cosmos DB Writer \- Striim, accessed August 29, 2025, [https://www.striim.com/docs/en/cosmos-db-writer.html](https://www.striim.com/docs/en/cosmos-db-writer.html)  
48. Best practices for Python SDK \- Azure Cosmos DB | Microsoft Learn, accessed August 29, 2025, [https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/best-practice-python](https://learn.microsoft.com/en-us/azure/cosmos-db/nosql/best-practice-python)  
49. How to pass credentials from kafka and database to FlinkSessionJob \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/79192921/how-to-pass-credentials-from-kafka-and-database-to-flinksessionjob](https://stackoverflow.com/questions/79192921/how-to-pass-credentials-from-kafka-and-database-to-flinksessionjob)  
50. Get Running with Apache Flink on Kubernetes, part 2 of 2 \- Decodable, accessed August 29, 2025, [https://www.decodable.co/blog/get-running-with-apache-flink-on-kubernetes-2](https://www.decodable.co/blog/get-running-with-apache-flink-on-kubernetes-2)  
51. Data Encryption in Confluent Manager for Apache Flink, accessed August 29, 2025, [https://docs.confluent.io/platform/current/flink/installation/encryption.html](https://docs.confluent.io/platform/current/flink/installation/encryption.html)  
52. Where to store credentials and other secrets for Apache Flink SQL? \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/69736740/where-to-store-credentials-and-other-secrets-for-apache-flink-sql](https://stackoverflow.com/questions/69736740/where-to-store-credentials-and-other-secrets-for-apache-flink-sql)  
53. Flink Configuration \- Ververica documentation, accessed August 29, 2025, [https://docs.ververica.com/vvp/2.11/user-guide/application-operations/deployments/flink-configuration/](https://docs.ververica.com/vvp/2.11/user-guide/application-operations/deployments/flink-configuration/)  
54. Secret Values \- Ververica documentation, accessed August 29, 2025, [https://docs.ververica.com/vvp/user-guide/application-operations/deployments/secret-values/](https://docs.ververica.com/vvp/user-guide/application-operations/deployments/secret-values/)  
55. Manage Connections in Confluent Cloud for Apache Flink, accessed August 29, 2025, [https://docs.confluent.io/cloud/current/flink/operate-and-deploy/manage-connections.html](https://docs.confluent.io/cloud/current/flink/operate-and-deploy/manage-connections.html)  
56. How do i pass Program Arguments From Apache Flink 2.0.0 Web Gui to my Job properly?, accessed August 29, 2025, [https://stackoverflow.com/questions/79702619/how-do-i-pass-program-arguments-from-apache-flink-2-0-0-web-gui-to-my-job-proper](https://stackoverflow.com/questions/79702619/how-do-i-pass-program-arguments-from-apache-flink-2-0-0-web-gui-to-my-job-proper)  
57. Apache Flink WebUI Set Environment Variables \- Stack Overflow, accessed August 29, 2025, [https://stackoverflow.com/questions/49184643/apache-flink-webui-set-environment-variables](https://stackoverflow.com/questions/49184643/apache-flink-webui-set-environment-variables)  
58. apache/flink-connector-jdbc \- GitHub, accessed August 29, 2025, [https://github.com/apache/flink-connector-jdbc](https://github.com/apache/flink-connector-jdbc)  
59. apache/flink-connector-prometheus \- GitHub, accessed August 29, 2025, [https://github.com/apache/flink-connector-prometheus](https://github.com/apache/flink-connector-prometheus)  
60. All You Need to Know About PyFlink \- Ververica, accessed August 29, 2025, [https://www.ververica.com/blog/all-you-need-to-know-about-pyflink](https://www.ververica.com/blog/all-you-need-to-know-about-pyflink)  
61. Apache Flink Streaming in Python \- Medium, accessed August 29, 2025, [https://medium.com/@priyankbhandia\_24919/apache-flink-for-data-enrichment-6118d48de04](https://medium.com/@priyankbhandia_24919/apache-flink-for-data-enrichment-6118d48de04)  
62. Python DataStream API Questions \-- Java/Scala Interoperability?, accessed August 29, 2025, [http://deprecated-apache-flink-user-mailing-list-archive.369.s1.nabble.com/Python-DataStream-API-Questions-Java-Scala-Interoperability-td41846.html](http://deprecated-apache-flink-user-mailing-list-archive.369.s1.nabble.com/Python-DataStream-API-Questions-Java-Scala-Interoperability-td41846.html)  
63. PyFlink — A deep dive into Flink's Python API \- Quix, accessed August 29, 2025, [https://quix.io/blog/pyflink-deep-dive](https://quix.io/blog/pyflink-deep-dive)  
64. Everything You Need to Know about PyFlink \- Alibaba Cloud Community, accessed August 29, 2025, [https://www.alibabacloud.com/blog/599959](https://www.alibabacloud.com/blog/599959)  
65. Table API \- Connectors \- 《Apache Flink v1.19 Documentation》 \- 书栈网 · BookStack, accessed August 29, 2025, [https://www.bookstack.cn/read/flink-1.19-en/2b4f4888908b20a4.md](https://www.bookstack.cn/read/flink-1.19-en/2b4f4888908b20a4.md)  
66. flink-extended/flink-scala-api \- GitHub, accessed August 29, 2025, [https://github.com/flink-extended/flink-scala-api](https://github.com/flink-extended/flink-scala-api)  
67. Flink Checkpoints — Best Practices (By FlinkPOD) | by VerticalServe Blogs \- Medium, accessed August 29, 2025, [https://verticalserve.medium.com/flink-checkpoints-best-practices-bf91dfc70f8f](https://verticalserve.medium.com/flink-checkpoints-best-practices-bf91dfc70f8f)