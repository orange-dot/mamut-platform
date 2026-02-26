package com.cosmos.flink.serialization

import com.cosmos.flink.models.{OrderCreated, ProcessedEvent}
import com.cosmos.flink.models.TypeInfos._
import com.fasterxml.jackson.databind.{ObjectMapper, DeserializationFeature}
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule
import com.fasterxml.jackson.module.scala.DefaultScalaModule
import org.apache.flink.api.common.serialization.DeserializationSchema
import org.apache.flink.api.common.typeinfo.TypeInformation

import scala.util.{Try, Success, Failure}
import scala.reflect.ClassTag

/**
 * Functional JSON deserializer using Scala's type system and error handling.
 * Demonstrates functional programming with Try monad and type-safe operations.
 */
class ScalaJacksonDeserializer[T: ClassTag](implicit typeInfo: TypeInformation[T]) extends DeserializationSchema[T] {
  
  @transient private lazy val objectMapper: ObjectMapper = {
    val mapper = new ObjectMapper()
    mapper.registerModule(DefaultScalaModule)
    mapper.registerModule(new JavaTimeModule())
    mapper.disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
    mapper
  }
  
  private val targetClass: Class[T] = {
    val classTag = implicitly[ClassTag[T]]
    classTag.runtimeClass.asInstanceOf[Class[T]]
  }
  
  /**
   * Functional deserialization with Try monad for error handling
   */
  override def deserialize(message: Array[Byte]): T = {
    deserializeSafe(message) match {
      case Success(value) => value
      case Failure(exception) => throw new RuntimeException(s"Failed to deserialize message", exception)
    }
  }
  
  /**
   * Safe deserialization returning Try for functional error handling
   */
  def deserializeSafe(message: Array[Byte]): Try[T] = Try {
    objectMapper.readValue(message, targetClass)
  }
  
  /**
   * Functional validation with Either for composable error handling
   */
  def deserializeWithValidation(message: Array[Byte]): Either[String, T] = {
    deserializeSafe(message) match {
      case Success(value) => Right(value)
      case Failure(exception) => Left(s"Deserialization failed: ${exception.getMessage}")
    }
  }
  
  override def isEndOfStream(nextElement: T): Boolean = false
  
  override def getProducedType: TypeInformation[T] = typeInfo
}

/**
 * Companion object with type-safe factory methods
 */
object ScalaJacksonDeserializer {
  
  /**
   * Factory method for OrderCreated with type inference
   */
  def forOrderCreated: ScalaJacksonDeserializer[OrderCreated] = 
    new ScalaJacksonDeserializer[OrderCreated]
  
  /**
   * Factory method for ProcessedEvent with type inference
   */
  def forProcessedEvent: ScalaJacksonDeserializer[ProcessedEvent] = 
    new ScalaJacksonDeserializer[ProcessedEvent]
  
  /**
   * Generic factory method with type parameter
   */
  def apply[T: ClassTag: TypeInformation]: ScalaJacksonDeserializer[T] = 
    new ScalaJacksonDeserializer[T]
}

/**
 * Functional JSON serializer using immutable operations and type safety
 */
class ScalaJacksonSerializer[T] extends org.apache.flink.api.common.serialization.SerializationSchema[T] {
  
  @transient private lazy val objectMapper: ObjectMapper = {
    val mapper = new ObjectMapper()
    mapper.registerModule(DefaultScalaModule)
    mapper.registerModule(new JavaTimeModule())
    mapper
  }
  
  /**
   * Functional serialization with error handling
   */
  override def serialize(element: T): Array[Byte] = {
    serializeSafe(element) match {
      case Success(bytes) => bytes
      case Failure(exception) => throw new RuntimeException(s"Failed to serialize element", exception)
    }
  }
  
  /**
   * Safe serialization returning Try for functional error handling
   */
  def serializeSafe(element: T): Try[Array[Byte]] = Try {
    objectMapper.writeValueAsBytes(element)
  }
  
  /**
   * Functional serialization with Either for composable error handling
   */
  def serializeWithValidation(element: T): Either[String, Array[Byte]] = {
    serializeSafe(element) match {
      case Success(bytes) => Right(bytes)
      case Failure(exception) => Left(s"Serialization failed: ${exception.getMessage}")
    }
  }
}

/**
 * Companion object with factory methods
 */
object ScalaJacksonSerializer {
  
  def forOrderCreated: ScalaJacksonSerializer[OrderCreated] = 
    new ScalaJacksonSerializer[OrderCreated]
  
  def forProcessedEvent: ScalaJacksonSerializer[ProcessedEvent] = 
    new ScalaJacksonSerializer[ProcessedEvent]
  
  def apply[T]: ScalaJacksonSerializer[T] = 
    new ScalaJacksonSerializer[T]
}

/**
 * Type-safe serialization utilities using Scala's type system
 */
object SerializationUtils {
  
  /**
   * Functional transformation with validation
   */
  def transformAndSerialize[A, B](
    input: A,
    transform: A => Either[String, B],
    serializer: ScalaJacksonSerializer[B]
  ): Either[String, Array[Byte]] = {
    for {
      transformed <- transform(input)
      serialized <- serializer.serializeWithValidation(transformed)
    } yield serialized
  }
  
  /**
   * Functional deserialization with validation
   */
  def deserializeAndTransform[A, B](
    input: Array[Byte],
    deserializer: ScalaJacksonDeserializer[A],
    transform: A => Either[String, B]
  ): Either[String, B] = {
    for {
      deserialized <- deserializer.deserializeWithValidation(input)
      transformed <- transform(deserialized)
    } yield transformed
  }
  
  /**
   * Pattern matching for type-safe serialization
   */
  def serializeByType(element: Any): Either[String, Array[Byte]] = element match {
    case order: OrderCreated => 
      ScalaJacksonSerializer.forOrderCreated.serializeWithValidation(order)
    case processed: ProcessedEvent => 
      ScalaJacksonSerializer.forProcessedEvent.serializeWithValidation(processed)
    case _ => 
      Left(s"Unsupported type: ${element.getClass.getSimpleName}")
  }
}