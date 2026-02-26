package com.cosmos.flink.sink

import com.cosmos.flink.config.{CosmosDBSinkConfig, ConsistencyLevel}
import com.cosmos.flink.models.ProcessedEvent
import com.cosmos.flink.serialization.ScalaJacksonSerializer
import org.apache.flink.api.connector.sink2._

import scala.util.{Try, Success, Failure}
import scala.concurrent.{Future, ExecutionContext}
import scala.collection.mutable.ListBuffer

/**
 * Functional Cosmos DB Sink implementation using Scala's functional programming features.
 * Demonstrates immutable state, pattern matching, and monadic operations for error handling.
 */
class ScalaCosmosDBSink[T](
  config: CosmosDBSinkConfig,
  serializer: ScalaJacksonSerializer[T]
)(implicit ec: ExecutionContext) extends Sink[T] {

  /**
   * Functional writer creation with immutable configuration
   */
  override def createWriter(context: Sink.InitContext): SinkWriter[T] = {
    new ScalaCosmosDBSinkWriter[T](config, serializer, context)
  }
}

/**
 * Functional sink writer using immutable batch operations
 */
class ScalaCosmosDBSinkWriter[T](
  config: CosmosDBSinkConfig,
  serializer: ScalaJacksonSerializer[T],
  context: Sink.InitContext
) extends SinkWriter[T] {
  
  private val buffer = ListBuffer[T]()
  private var lastFlushTime: Long = System.currentTimeMillis()
  
  /**
   * Functional element writing with immutable batching
   */
  override def write(element: T, context: SinkWriter.Context): Unit = {
    buffer += element
    
    // Pattern matching for flush conditions
    val shouldFlush = (buffer.size >= config.batchSize, System.currentTimeMillis() - lastFlushTime > config.batchTimeoutMs) match {
      case (true, _) => true
      case (_, true) => true
      case _ => false
    }
    
    if (shouldFlush) flush(endOfInput = false)
  }
  
  /**
   * Functional flush implementation with Either for error handling
   */
  override def flush(endOfInput: Boolean): Unit = {
    if (buffer.nonEmpty || endOfInput) {
      val elementsToFlush = buffer.toList
      buffer.clear()
      lastFlushTime = System.currentTimeMillis()
      
      // Functional serialization with error handling
      val serializedResults = elementsToFlush.map(serializer.serializeWithValidation)
      
      // Pattern matching for batch processing
      val (successes, failures) = serializedResults.zipWithIndex.partition(_._1.isRight)
      
      if (failures.nonEmpty) {
        val errorMessages = failures.collect { case (Left(error), index) => 
          s"Element $index: $error" 
        }.mkString("; ")
        throw new RuntimeException(s"Serialization failures: $errorMessages")
      }
      
      // Extract successful serializations  
      val documents = successes.map(_._1.getOrElse(Array.empty)).map(new String(_, "UTF-8"))
      
      // Simulate processing (in real implementation, this would write to Cosmos DB)
      Try {
        println(s"Processing batch of ${documents.size} documents")
        Thread.sleep(10) // Simulate async operation
      } match {
        case Success(_) => // Success case
        case Failure(exception) => 
          throw new RuntimeException("Failed to process batch", exception)
      }
    }
  }
  
  override def close(): Unit = {
    flush(endOfInput = true)
  }
}

/**
 * Companion objects with functional factory methods
 */
object ScalaCosmosDBSink {
  
  /**
   * Factory method for ProcessedEvent sink with defaults
   */
  def forProcessedEvent(config: CosmosDBSinkConfig)(implicit ec: ExecutionContext): Either[String, ScalaCosmosDBSink[ProcessedEvent]] = {
    config.validate.map(_ => new ScalaCosmosDBSink(config, ScalaJacksonSerializer.forProcessedEvent))
      .left.map(errors => s"Invalid configuration: ${errors.mkString(", ")}")
  }
}