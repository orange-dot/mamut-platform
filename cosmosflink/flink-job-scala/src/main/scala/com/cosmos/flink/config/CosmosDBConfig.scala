package com.cosmos.flink.config

import scala.util.{Try, Success, Failure}

/**
 * Immutable case class for Cosmos DB Source configuration using functional programming principles.
 * Demonstrates type safety, immutability, and functional error handling.
 */
case class CosmosDBSourceConfig(
  endpoint: String,
  key: String,
  databaseName: String,
  containerName: String,
  leaseContainerName: String,
  maxItemCount: Int = 100,
  preferredRegions: List[String] = List.empty,
  changeFeedMode: ChangeFeedMode = ChangeFeedMode.LatestVersion,
  pollInterval: scala.concurrent.duration.Duration = scala.concurrent.duration.Duration("5 seconds")
) {
  
  /**
   * Functional validation using Either for composable error handling
   */
  def validate: Either[List[String], CosmosDBSourceConfig] = {
    val errors = List(
      if (endpoint.trim.isEmpty) Some("Endpoint cannot be empty") else None,
      if (key.trim.isEmpty) Some("Key cannot be empty") else None,
      if (databaseName.trim.isEmpty) Some("Database name cannot be empty") else None,
      if (containerName.trim.isEmpty) Some("Container name cannot be empty") else None,
      if (leaseContainerName.trim.isEmpty) Some("Lease container name cannot be empty") else None,
      if (maxItemCount <= 0) Some("Max item count must be positive") else None
    ).flatten
    
    if (errors.isEmpty) Right(this) else Left(errors)
  }
  
  /**
   * Functional composition for configuration updates
   */
  def withMaxItemCount(count: Int): CosmosDBSourceConfig = copy(maxItemCount = count)
  def withPreferredRegions(regions: List[String]): CosmosDBSourceConfig = copy(preferredRegions = regions)
  def withChangeFeedMode(mode: ChangeFeedMode): CosmosDBSourceConfig = copy(changeFeedMode = mode)
}

/**
 * Sealed trait for type-safe change feed modes using ADT (Algebraic Data Type)
 */
sealed trait ChangeFeedMode

object ChangeFeedMode {
  case object LatestVersion extends ChangeFeedMode
  case object FromBeginning extends ChangeFeedMode
  case class FromPointInTime(timestamp: java.time.Instant) extends ChangeFeedMode
}

/**
 * Immutable case class for Cosmos DB Sink configuration with functional validation
 */
case class CosmosDBSinkConfig(
  endpoint: String,
  key: String,
  databaseName: String,
  containerName: String,
  batchSize: Int = 25,
  batchTimeoutMs: Long = 5000,
  maxRetryAttempts: Int = 3,
  consistencyLevel: ConsistencyLevel = ConsistencyLevel.Session
) {
  
  /**
   * Functional validation with comprehensive error collection
   */
  def validate: Either[List[String], CosmosDBSinkConfig] = {
    val errors = List(
      if (endpoint.trim.isEmpty) Some("Endpoint cannot be empty") else None,
      if (key.trim.isEmpty) Some("Key cannot be empty") else None,
      if (databaseName.trim.isEmpty) Some("Database name cannot be empty") else None,
      if (containerName.trim.isEmpty) Some("Container name cannot be empty") else None,
      if (batchSize <= 0) Some("Batch size must be positive") else None,
      if (batchTimeoutMs <= 0) Some("Batch timeout must be positive") else None,
      if (maxRetryAttempts < 0) Some("Max retry attempts cannot be negative") else None
    ).flatten
    
    if (errors.isEmpty) Right(this) else Left(errors)
  }
  
  /**
   * Functional updates with immutable data
   */
  def withBatchSize(size: Int): CosmosDBSinkConfig = copy(batchSize = size)
  def withBatchTimeout(timeout: Long): CosmosDBSinkConfig = copy(batchTimeoutMs = timeout)
  def withConsistencyLevel(level: ConsistencyLevel): CosmosDBSinkConfig = copy(consistencyLevel = level)
}

/**
 * Sealed trait for type-safe consistency levels
 */
sealed trait ConsistencyLevel

object ConsistencyLevel {
  case object Strong extends ConsistencyLevel
  case object BoundedStaleness extends ConsistencyLevel
  case object Session extends ConsistencyLevel
  case object ConsistentPrefix extends ConsistencyLevel
  case object Eventual extends ConsistencyLevel
}

/**
 * Companion objects with functional factory methods and environment loading
 */
object CosmosDBSourceConfig {
  
  /**
   * Load configuration from environment variables using Try monad for error handling
   */
  def fromEnvironment: Try[CosmosDBSourceConfig] = Try {
    CosmosDBSourceConfig(
      endpoint = sys.env("COSMOS_ENDPOINT"),
      key = sys.env("COSMOS_KEY"),
      databaseName = sys.env("COSMOS_DATABASE"),
      containerName = sys.env("COSMOS_SOURCE_CONTAINER"),
      leaseContainerName = sys.env("COSMOS_LEASE_CONTAINER"),
      maxItemCount = sys.env.get("COSMOS_MAX_ITEM_COUNT").map(_.toInt).getOrElse(100),
      preferredRegions = sys.env.get("COSMOS_PREFERRED_REGIONS")
        .map(_.split(",").toList.map(_.trim))
        .getOrElse(List.empty)
    )
  }.flatMap(config => config.validate.fold(
    errors => Failure(new IllegalArgumentException(s"Configuration validation failed: ${errors.mkString(", ")}")),
    validConfig => Success(validConfig)
  ))
  
  /**
   * Builder pattern with functional composition
   */
  def builder: CosmosDBSourceConfigBuilder = CosmosDBSourceConfigBuilder()
}

object CosmosDBSinkConfig {
  
  /**
   * Load sink configuration from environment using functional error handling
   */
  def fromEnvironment: Try[CosmosDBSinkConfig] = Try {
    CosmosDBSinkConfig(
      endpoint = sys.env("COSMOS_ENDPOINT"),
      key = sys.env("COSMOS_KEY"),
      databaseName = sys.env("COSMOS_DATABASE"),
      containerName = sys.env("COSMOS_SINK_CONTAINER"),
      batchSize = sys.env.get("COSMOS_SINK_BATCH_SIZE").map(_.toInt).getOrElse(25),
      batchTimeoutMs = sys.env.get("COSMOS_SINK_BATCH_TIMEOUT").map(_.toLong).getOrElse(5000),
      maxRetryAttempts = sys.env.get("COSMOS_SINK_MAX_RETRIES").map(_.toInt).getOrElse(3)
    )
  }.flatMap(config => config.validate.fold(
    errors => Failure(new IllegalArgumentException(s"Configuration validation failed: ${errors.mkString(", ")}")),
    validConfig => Success(validConfig)
  ))
  
  def builder: CosmosDBSinkConfigBuilder = CosmosDBSinkConfigBuilder()
}

/**
 * Builder pattern with functional composition and immutable state
 */
case class CosmosDBSourceConfigBuilder(
  endpoint: Option[String] = None,
  key: Option[String] = None,
  databaseName: Option[String] = None,
  containerName: Option[String] = None,
  leaseContainerName: Option[String] = None,
  maxItemCount: Int = 100,
  preferredRegions: List[String] = List.empty,
  changeFeedMode: ChangeFeedMode = ChangeFeedMode.LatestVersion,
  pollInterval: scala.concurrent.duration.Duration = scala.concurrent.duration.Duration("5 seconds")
) {
  
  def endpoint(value: String): CosmosDBSourceConfigBuilder = copy(endpoint = Some(value))
  def key(value: String): CosmosDBSourceConfigBuilder = copy(key = Some(value))
  def databaseName(value: String): CosmosDBSourceConfigBuilder = copy(databaseName = Some(value))
  def containerName(value: String): CosmosDBSourceConfigBuilder = copy(containerName = Some(value))
  def leaseContainerName(value: String): CosmosDBSourceConfigBuilder = copy(leaseContainerName = Some(value))
  def maxItemCount(value: Int): CosmosDBSourceConfigBuilder = copy(maxItemCount = value)
  def preferredRegions(value: List[String]): CosmosDBSourceConfigBuilder = copy(preferredRegions = value)
  def changeFeedMode(value: ChangeFeedMode): CosmosDBSourceConfigBuilder = copy(changeFeedMode = value)
  def pollInterval(value: scala.concurrent.duration.Duration): CosmosDBSourceConfigBuilder = copy(pollInterval = value)
  
  /**
   * Build with functional validation
   */
  def build: Either[String, CosmosDBSourceConfig] = {
    for {
      ep <- endpoint.toRight("Endpoint is required")
      k <- key.toRight("Key is required")
      db <- databaseName.toRight("Database name is required")
      container <- containerName.toRight("Container name is required")
      lease <- leaseContainerName.toRight("Lease container name is required")
      config = CosmosDBSourceConfig(ep, k, db, container, lease, maxItemCount, preferredRegions, changeFeedMode, pollInterval)
      validConfig <- config.validate.left.map(_.mkString(", "))
    } yield validConfig
  }
}

case class CosmosDBSinkConfigBuilder(
  endpoint: Option[String] = None,
  key: Option[String] = None,
  databaseName: Option[String] = None,
  containerName: Option[String] = None,
  batchSize: Int = 25,
  batchTimeoutMs: Long = 5000,
  maxRetryAttempts: Int = 3,
  consistencyLevel: ConsistencyLevel = ConsistencyLevel.Session
) {
  
  def endpoint(value: String): CosmosDBSinkConfigBuilder = copy(endpoint = Some(value))
  def key(value: String): CosmosDBSinkConfigBuilder = copy(key = Some(value))
  def databaseName(value: String): CosmosDBSinkConfigBuilder = copy(databaseName = Some(value))
  def containerName(value: String): CosmosDBSinkConfigBuilder = copy(containerName = Some(value))
  def batchSize(value: Int): CosmosDBSinkConfigBuilder = copy(batchSize = value)
  def batchTimeoutMs(value: Long): CosmosDBSinkConfigBuilder = copy(batchTimeoutMs = value)
  def maxRetryAttempts(value: Int): CosmosDBSinkConfigBuilder = copy(maxRetryAttempts = value)
  def consistencyLevel(value: ConsistencyLevel): CosmosDBSinkConfigBuilder = copy(consistencyLevel = value)
  
  def build: Either[String, CosmosDBSinkConfig] = {
    for {
      ep <- endpoint.toRight("Endpoint is required")
      k <- key.toRight("Key is required")
      db <- databaseName.toRight("Database name is required")
      container <- containerName.toRight("Container name is required")
      config = CosmosDBSinkConfig(ep, k, db, container, batchSize, batchTimeoutMs, maxRetryAttempts, consistencyLevel)
      validConfig <- config.validate.left.map(_.mkString(", "))
    } yield validConfig
  }
}