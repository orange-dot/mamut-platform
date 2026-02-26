package com.cosmos.flink

import com.cosmos.flink.config.{CosmosDBSourceConfig, CosmosDBSinkConfig}
import com.cosmos.flink.models.{OrderCreated, ProcessedEvent}
import com.cosmos.flink.models.TypeInfos._
import com.cosmos.flink.source.ScalaCosmosDBSource
import com.cosmos.flink.sink.ScalaCosmosDBSink
import com.cosmos.flink.serialization.{ScalaJacksonDeserializer, ScalaJacksonSerializer}

import org.apache.flink.streaming.api.scala._
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment
import org.apache.flink.streaming.api.CheckpointingMode
import org.apache.flink.runtime.state.hashmap.HashMapStateBackend
import org.apache.flink.api.common.eventtime.WatermarkStrategy
import org.apache.flink.api.common.typeinfo.TypeInformation

import scala.util.{Try, Success, Failure}
import scala.concurrent.ExecutionContext
import java.time.LocalDateTime

/**
 * Main Flink job demonstrating functional programming with Scala.
 * Showcases immutable data structures, pattern matching, and monadic operations.
 */
object ScalaFlinkCosmosJob {
  
  implicit val ec: ExecutionContext = ExecutionContext.global
  
  def main(args: Array[String]): Unit = {
    
    // Functional environment setup with error handling
    val jobResult = for {
      env <- createEnvironment
      sourceConfig <- loadSourceConfig
      sinkConfig <- loadSinkConfig
      _ <- runPipeline(env, sourceConfig, sinkConfig)
    } yield ()
    
    // Pattern matching for error handling
    jobResult match {
      case Success(_) => 
        println("✅ Scala Flink Cosmos DB job completed successfully")
      case Failure(exception) => 
        println(s"❌ Job failed: ${exception.getMessage}")
        exception.printStackTrace()
        System.exit(1)
    }
  }
  
  /**
   * Functional environment creation with immutable configuration
   */
  private def createEnvironment: Try[StreamExecutionEnvironment] = Try {
    val env = StreamExecutionEnvironment.getExecutionEnvironment
    
    // Functional configuration with method chaining
    env.enableCheckpointing(10000, CheckpointingMode.EXACTLY_ONCE)
    env.setStateBackend(new HashMapStateBackend())
    env.getCheckpointConfig.setMinPauseBetweenCheckpoints(5000)
    env.getCheckpointConfig.setCheckpointTimeout(60000)
    env.getCheckpointConfig.setMaxConcurrentCheckpoints(1)
    
    env
  }
  
  /**
   * Functional configuration loading with Either for error handling
   */
  private def loadSourceConfig: Try[CosmosDBSourceConfig] = {
    CosmosDBSourceConfig.fromEnvironment
  }
  
  private def loadSinkConfig: Try[CosmosDBSinkConfig] = {
    CosmosDBSinkConfig.fromEnvironment
  }
  
  /**
   * Main pipeline implementation using functional composition
   */
  private def runPipeline(
    env: StreamExecutionEnvironment,
    sourceConfig: CosmosDBSourceConfig,
    sinkConfig: CosmosDBSinkConfig
  ): Try[Unit] = Try {
    
    // Create source with functional builder pattern
    val sourceResult = ScalaCosmosDBSource.forOrderCreated(sourceConfig)
    val cosmosSource = sourceResult match {
      case Right(source) => source
      case Left(error) => throw new RuntimeException(s"Failed to create source: $error")
    }
    
    // Create sink with functional builder pattern  
    val sinkResult = ScalaCosmosDBSink.forProcessedEvent(sinkConfig)
    val cosmosSink = sinkResult match {
      case Right(sink) => sink
      case Left(error) => throw new RuntimeException(s"Failed to create sink: $error")
    }
    
    // Functional data stream processing with immutable transformations
    val orderStream: DataStream[OrderCreated] = env
      .fromSource(cosmosSource, WatermarkStrategy.noWatermarks(), "Cosmos DB Source")
      .asInstanceOf[DataStream[OrderCreated]]
      .setParallelism(4)
    
    // Functional transformation pipeline using pattern matching and immutable operations
    val processedStream: DataStream[ProcessedEvent] = orderStream
      .filter(_.isValid) // Pattern matching validation
      .map(transformOrder _) // Functional transformation
      .map(enrichEvent _) // Functional enrichment
      .filter(_.isSuccessful) // Pattern matching for success
    
    // Sink with functional composition
    processedStream
      .sinkTo(cosmosSink)
      .setParallelism(2)
      .name("Cosmos DB Sink")
    
    // Execute with functional error handling
    env.execute("Scala Flink Cosmos DB Pipeline")
  }
  
  /**
   * Functional order transformation using immutable operations
   */
  private def transformOrder(order: OrderCreated): ProcessedEvent = {
    // Pattern matching for order categorization
    val processingStatus = order match {
      case OrderCreated(_, _, _, amount, items, _) if amount > 1000 && items.length > 5 => 
        "HIGH_VALUE_BULK"
      case OrderCreated(_, _, _, amount, _, _) if amount > 1000 => 
        "HIGH_VALUE"
      case OrderCreated(_, _, _, _, items, _) if items.length > 5 => 
        "BULK_ORDER"
      case _ => 
        "STANDARD"
    }
    
    // Functional transformation with immutable data
    order.toProcessedEvent.copy(processingStatus = processingStatus)
  }
  
  /**
   * Functional event enrichment using case class operations
   */
  private def enrichEvent(event: ProcessedEvent): ProcessedEvent = {
    // Pattern matching for region-based enrichment
    val enrichedStatus = event.region match {
      case "US" | "CA" => s"${event.processingStatus}_AMERICAS"
      case "EU" | "UK" => s"${event.processingStatus}_EUROPE" 
      case "APAC" | "AU" => s"${event.processingStatus}_ASIA_PACIFIC"
      case _ => s"${event.processingStatus}_GLOBAL"
    }
    
    // Immutable update using case class copy
    event.copy(
      processingStatus = enrichedStatus,
      processedAt = LocalDateTime.now()
    )
  }
}

/**
 * Functional utilities for stream processing using higher-order functions
 */
object StreamProcessingUtils {
  
  /**
   * Higher-order function for functional filtering with validation
   */
  def filterWithValidation[T](
    stream: DataStream[T],
    validator: T => Boolean,
    errorHandler: T => Unit = (_: T) => ()
  ): DataStream[T] = {
    stream.filter { (element: T) =>
      val isValid = validator(element)
      if (!isValid) errorHandler(element)
      isValid
    }
  }
  
  /**
   * Functional transformation with Either for error handling
   */
  def transformSafely[A, B](
    stream: DataStream[A],
    transform: A => Either[String, B]
  )(implicit typeInfo: TypeInformation[B]): DataStream[B] = {
    stream
      .map(transform)
      .filter((result: Either[String, B]) => result.isRight)
      .map((result: Either[String, B]) => result.getOrElse(throw new RuntimeException("Should not happen")))
  }
  
  /**
   * Pattern matching for stream routing
   */
  def routeByPattern[T](
    stream: DataStream[T],
    patterns: Map[T => Boolean, String]
  ): Map[String, DataStream[T]] = {
    patterns.map { case (predicate, name) =>
      name -> stream.filter(predicate).name(s"Route: $name")
    }
  }
  
  /**
   * Functional aggregation with windowing
   */
  def aggregateWithWindow[T, K, R](
    stream: DataStream[T],
    keySelector: T => K,
    aggregator: List[T] => R,
    windowSize: org.apache.flink.streaming.api.windowing.time.Time
  )(implicit keyTypeInfo: TypeInformation[K], resultTypeInfo: TypeInformation[R], listTypeInfo: TypeInformation[List[T]]): DataStream[R] = {
    stream
      .keyBy(keySelector)
      .timeWindow(windowSize)
      .aggregate(new FunctionalAggregateFunction(aggregator))
  }
}

/**
 * Functional aggregate function using higher-order functions
 */
class FunctionalAggregateFunction[T, R](aggregator: List[T] => R) 
  extends org.apache.flink.api.common.functions.AggregateFunction[T, List[T], R] {
  
  override def createAccumulator(): List[T] = List.empty
  
  override def add(value: T, accumulator: List[T]): List[T] = value :: accumulator
  
  override def getResult(accumulator: List[T]): R = aggregator(accumulator.reverse)
  
  override def merge(a: List[T], b: List[T]): List[T] = a ++ b
}