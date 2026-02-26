package com.cosmos.flink.models

import com.fasterxml.jackson.annotation.JsonProperty
import com.fasterxml.jackson.databind.annotation.JsonDeserialize
import com.fasterxml.jackson.datatype.jsr310.deser.LocalDateTimeDeserializer

import java.time.LocalDateTime

/**
 * Immutable case class representing an order creation event in Cosmos DB.
 * Uses functional programming principles with immutable data structures.
 */
case class OrderCreated(
  id: String,
  customerId: String,
  orderDate: LocalDateTime,
  totalAmount: BigDecimal,
  items: List[OrderItem],
  region: String
) {
  /**
   * Functional transformation to ProcessedEvent using case class copy method
   */
  def toProcessedEvent: ProcessedEvent = ProcessedEvent(
    id = id,
    originalOrderId = id,
    customerId = customerId,
    processedAt = LocalDateTime.now(),
    totalAmount = totalAmount,
    itemCount = items.length,
    region = region,
    processingStatus = "PROCESSED"
  )
  
  /**
   * Pattern matching for order validation
   */
  def isValid: Boolean = this match {
    case OrderCreated(id, customerId, _, amount, items, region) 
      if id.nonEmpty && customerId.nonEmpty && amount > 0 && items.nonEmpty && region.nonEmpty => true
    case _ => false
  }
  
  /**
   * Functional composition for order enrichment
   */
  def withRegion(newRegion: String): OrderCreated = copy(region = newRegion)
  def addItem(item: OrderItem): OrderCreated = copy(items = items :+ item)
}

/**
 * Immutable case class for order items with functional operations
 */
case class OrderItem(
  productId: String,
  productName: String,
  quantity: Int,
  price: BigDecimal
) {
  def totalPrice: BigDecimal = price * quantity
  
  // Pattern matching for item validation
  def isValid: Boolean = this match {
    case OrderItem(id, name, qty, price) 
      if id.nonEmpty && name.nonEmpty && qty > 0 && price > 0 => true
    case _ => false
  }
}

/**
 * Immutable case class for processed events with functional transformations
 */
case class ProcessedEvent(
  id: String,
  originalOrderId: String,
  customerId: String,
  processedAt: LocalDateTime,
  totalAmount: BigDecimal,
  itemCount: Int,
  region: String,
  processingStatus: String
) {
  /**
   * Functional status updates using copy
   */
  def markCompleted: ProcessedEvent = copy(processingStatus = "COMPLETED")
  def markFailed(reason: String): ProcessedEvent = copy(processingStatus = s"FAILED: $reason")
  
  /**
   * Pattern matching for status validation
   */
  def isSuccessful: Boolean = processingStatus match {
    case "PROCESSED" | "COMPLETED" => true
    case _ => false
  }
}

/**
 * Companion objects with factory methods and type-safe constructors
 */
object OrderCreated {
  /**
   * Safe constructor with Either for error handling
   */
  def create(
    id: String,
    customerId: String,
    orderDate: LocalDateTime,
    totalAmount: BigDecimal,
    items: List[OrderItem],
    region: String
  ): Either[String, OrderCreated] = {
    val order = OrderCreated(id, customerId, orderDate, totalAmount, items, region)
    if (order.isValid) Right(order)
    else Left("Invalid order data")
  }
}

object OrderItem {
  /**
   * Safe constructor with validation
   */
  def create(
    productId: String,
    productName: String,
    quantity: Int,
    price: BigDecimal
  ): Either[String, OrderItem] = {
    val item = OrderItem(productId, productName, quantity, price)
    if (item.isValid) Right(item)
    else Left("Invalid item data")
  }
}

object ProcessedEvent {
  /**
   * Factory method from OrderCreated with functional transformation
   */
  def fromOrder(order: OrderCreated): ProcessedEvent = order.toProcessedEvent
}

// TypeInformation implicits for Flink serialization
import org.apache.flink.api.common.typeinfo.TypeInformation
import org.apache.flink.api.scala.typeutils.Types

object TypeInfos {
  implicit val orderCreatedTypeInfo: TypeInformation[OrderCreated] = Types.CASE_CLASS[OrderCreated]
  implicit val processedEventTypeInfo: TypeInformation[ProcessedEvent] = Types.CASE_CLASS[ProcessedEvent]
  implicit val orderItemTypeInfo: TypeInformation[OrderItem] = Types.CASE_CLASS[OrderItem]
}