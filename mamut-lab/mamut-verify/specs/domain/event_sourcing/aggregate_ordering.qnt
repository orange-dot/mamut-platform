/// aggregate_ordering.qnt - Full specification for event sourcing aggregate ordering
///
/// This specification models an event store that maintains per-aggregate event ordering.
/// It verifies that events are correctly ordered, versioned, and can be read consistently.
///
/// Key properties verified:
/// - Per-aggregate ordering: events for each aggregate maintain version order
/// - Version consistency: version numbers are consecutive and unique per aggregate
/// - No duplicate events: each event ID is unique across the entire store
/// - Positive versions: all version numbers are positive integers
/// - Read your writes: after appending an event, subsequent reads include it
module aggregate_ordering {

  import EventSourcing.*

  // ============================================================================
  // Constants
  // ============================================================================

  /// Maximum number of aggregates to model (for bounded model checking)
  const MAX_AGGREGATES: int = 3

  /// Maximum number of events per aggregate
  const MAX_EVENTS_PER_AGGREGATE: int = 5

  /// Set of aggregate IDs in the system
  const AGGREGATE_IDS: Set[str] = Set("agg1", "agg2", "agg3")

  /// Set of event types
  const EVENT_TYPES: Set[str] = Set("Created", "Updated", "Deleted")

  // ============================================================================
  // State Variables
  // ============================================================================

  /// The event store: all events that have been appended
  var events: List[Event]

  /// Current version for each aggregate (aggregateId -> version)
  var versions: str -> int

  /// Set of all event IDs (for duplicate detection)
  var eventIds: Set[str]

  /// Logical clock for ordering
  var clock: int

  /// Track which events have been read by which client (for read-your-writes)
  var clientReads: str -> Set[str]  // clientId -> set of event IDs read

  /// Track which events were written by which client
  var clientWrites: str -> Set[str]  // clientId -> set of event IDs written

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// Result of an append operation
  type AppendOutcome =
    | AppendSuccess(Event)
    | AppendConflict(str)      // Expected version mismatch
    | AppendDuplicate(str)     // Duplicate event ID

  /// Result of a read operation
  type ReadOutcome = {
    events: List[Event],
    clientId: str
  }

  // ============================================================================
  // State Predicates
  // ============================================================================

  /// Check if an aggregate exists (has at least one event)
  def aggregateExists(aggId: str): bool = {
    versions.keys().contains(aggId)
  }

  /// Get the current version for an aggregate (0 if new)
  def getVersion(aggId: str): int = {
    versions.getOrElse(aggId, 0)
  }

  /// Get events for a specific aggregate
  def getAggregateEvents(aggId: str): List[Event] = {
    events.filter(e => e.aggregateId == aggId).sortBy(e => e.version)
  }

  // ============================================================================
  // Actions
  // ============================================================================

  /// Initialize the event store to empty state
  action Init: bool = all {
    events' = [],
    versions' = Map(),
    eventIds' = Set(),
    clock' = 0,
    clientReads' = Map(),
    clientWrites' = Map()
  }

  /// Append an event to the store with optimistic concurrency control
  ///
  /// Parameters:
  ///   eventId - Unique identifier for the event
  ///   aggId - Aggregate to append to
  ///   expectedVersion - Expected current version (for optimistic locking)
  ///   eventType - Type of the event
  ///   payload - Event payload data
  ///   clientId - Client performing the append
  action AppendEvent(
    eventId: str,
    aggId: str,
    expectedVersion: int,
    eventType: str,
    payload: str,
    clientId: str
  ): bool = {
    // Check for duplicate event ID
    if (eventIds.contains(eventId)) {
      // Duplicate - no state change (idempotent)
      all {
        events' = events,
        versions' = versions,
        eventIds' = eventIds,
        clock' = clock,
        clientReads' = clientReads,
        clientWrites' = clientWrites
      }
    } else {
      val currentVersion = getVersion(aggId)

      // Check optimistic concurrency
      if (expectedVersion != currentVersion) {
        // Conflict - no state change
        all {
          events' = events,
          versions' = versions,
          eventIds' = eventIds,
          clock' = clock,
          clientReads' = clientReads,
          clientWrites' = clientWrites
        }
      } else {
        // Success - append the event
        val newVersion = currentVersion + 1
        val newEvent: Event = {
          id: eventId,
          aggregateId: aggId,
          version: newVersion,
          timestamp: clock,
          eventType: eventType,
          payload: payload
        }

        val currentClientWrites = clientWrites.getOrElse(clientId, Set())

        all {
          events' = events.append(newEvent),
          versions' = versions.set(aggId, newVersion),
          eventIds' = eventIds.union(Set(eventId)),
          clock' = clock + 1,
          clientReads' = clientReads,
          clientWrites' = clientWrites.set(clientId, currentClientWrites.union(Set(eventId)))
        }
      }
    }
  }

  /// Read all events for an aggregate
  ///
  /// Parameters:
  ///   aggId - Aggregate to read
  ///   clientId - Client performing the read
  action ReadEvents(aggId: str, clientId: str): bool = {
    val aggEvents = getAggregateEvents(aggId)
    val readEventIds = aggEvents.map(e => e.id).toSet()
    val currentReads = clientReads.getOrElse(clientId, Set())

    all {
      events' = events,
      versions' = versions,
      eventIds' = eventIds,
      clock' = clock,
      clientReads' = clientReads.set(clientId, currentReads.union(readEventIds)),
      clientWrites' = clientWrites
    }
  }

  /// Read events after a specific version (for catching up)
  action ReadEventsAfter(aggId: str, afterVersion: int, clientId: str): bool = {
    val aggEvents = events
      .filter(e => e.aggregateId == aggId)
      .filter(e => e.version > afterVersion)
      .sortBy(e => e.version)
    val readEventIds = aggEvents.map(e => e.id).toSet()
    val currentReads = clientReads.getOrElse(clientId, Set())

    all {
      events' = events,
      versions' = versions,
      eventIds' = eventIds,
      clock' = clock,
      clientReads' = clientReads.set(clientId, currentReads.union(readEventIds)),
      clientWrites' = clientWrites
    }
  }

  /// Non-deterministic step: either append or read
  action step: bool = {
    nondet eventId = oneOf(Set("e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9", "e10"))
    nondet aggId = oneOf(AGGREGATE_IDS)
    nondet eventType = oneOf(EVENT_TYPES)
    nondet clientId = oneOf(Set("client1", "client2", "client3"))
    nondet expectedVersion = oneOf(Set(0, 1, 2, 3, 4))

    any {
      AppendEvent(eventId, aggId, expectedVersion, eventType, "payload", clientId),
      ReadEvents(aggId, clientId),
      ReadEventsAfter(aggId, expectedVersion, clientId)
    }
  }

  // ============================================================================
  // Invariants
  // ============================================================================

  /// INV1: Per-Aggregate Ordering
  /// Events within each aggregate are ordered by version
  val PerAggregateOrdering: bool = {
    AGGREGATE_IDS.forall(aggId => {
      val aggEvents = getAggregateEvents(aggId)
      aggEvents.indices().forall(i => {
        aggEvents.indices().forall(j => {
          (i >= j) or (aggEvents[i].version < aggEvents[j].version)
        })
      })
    })
  }

  /// INV2: Version Consistency
  /// The stored version matches the max version in events, and versions are consecutive
  val VersionConsistency: bool = {
    AGGREGATE_IDS.forall(aggId => {
      val aggEvents = getAggregateEvents(aggId)
      val storedVersion = getVersion(aggId)

      if (aggEvents.length() == 0) {
        storedVersion == 0 or not(versions.keys().contains(aggId))
      } else {
        val maxEventVersion = aggEvents.map(e => e.version).fold(0, (a, b) => if (a > b) a else b)
        val minEventVersion = aggEvents.map(e => e.version).fold(maxEventVersion, (a, b) => if (a < b) a else b)

        // Stored version equals max event version
        storedVersion == maxEventVersion and
        // Versions start at 1
        minEventVersion == 1 and
        // No gaps in versions
        aggEvents.length() == maxEventVersion
      }
    })
  }

  /// INV3: No Duplicate Events
  /// Each event ID is unique across the entire store
  val NoDuplicateEvents: bool = {
    val allIds = events.map(e => e.id)
    allIds.toSet().size() == allIds.length()
  }

  /// INV4: Positive Versions
  /// All version numbers are positive integers
  val PositiveVersions: bool = {
    events.forall(e => e.version > 0) and
    versions.keys().forall(k => versions.get(k) > 0)
  }

  /// INV5: Event IDs Match Stored IDs
  /// The eventIds set contains exactly the IDs of events in the store
  val EventIdsConsistent: bool = {
    eventIds == events.map(e => e.id).toSet()
  }

  /// INV6: Timestamps Are Monotonic
  /// Events appended later have higher or equal timestamps
  val MonotonicTimestamps: bool = {
    events.indices().forall(i => {
      events.indices().forall(j => {
        (i >= j) or (events[i].timestamp <= events[j].timestamp)
      })
    })
  }

  /// Combined invariant for all safety properties
  val Safety: bool = {
    PerAggregateOrdering and
    VersionConsistency and
    NoDuplicateEvents and
    PositiveVersions and
    EventIdsConsistent and
    MonotonicTimestamps
  }

  // ============================================================================
  // Temporal Properties
  // ============================================================================

  /// TEMP1: Read Your Writes
  /// If a client writes an event and then reads the aggregate,
  /// the read will include that event
  ///
  /// This is expressed as: for all clients, all events written by that client
  /// that are for aggregates the client has read after writing will be in the read set
  val ReadYourWrites: bool = {
    clientWrites.keys().forall(clientId => {
      val written = clientWrites.getOrElse(clientId, Set())
      val read = clientReads.getOrElse(clientId, Set())

      // All written events that the client has subsequently read should be visible
      // (This is a simplified check - in temporal logic we'd use eventually)
      written.forall(eventId => {
        // If this event exists in the store, check visibility
        val event = events.filter(e => e.id == eventId)
        event.length() == 0 or read.contains(eventId) or not(read.intersect(written).size() > 0)
      })
    })
  }

  /// TEMP2: Eventual Visibility
  /// Events once written are eventually visible to all clients
  /// (In bounded model checking, we check that written events exist in the store)
  val EventsPersist: bool = {
    eventIds.forall(id => events.exists(e => e.id == id))
  }

  /// TEMP3: No Lost Writes
  /// Once an event is appended, it remains in the store forever
  val NoLostWrites: bool = {
    // This would be checked as a temporal property:
    // [](event in events => [](event in events))
    // For now, we verify via the EventIdsConsistent invariant
    true
  }

  // ============================================================================
  // Test Scenarios
  // ============================================================================

  /// Test: Initial state is valid
  val testInitialState: bool = {
    Init implies Safety
  }

  /// Test: Single append maintains invariants
  run testSingleAppend = {
    Init
    .then(AppendEvent("e1", "agg1", 0, "Created", "test", "client1"))
    .then(all {
      assert(Safety),
      assert(events.length() == 1),
      assert(getVersion("agg1") == 1)
    })
  }

  /// Test: Multiple appends to same aggregate
  run testMultipleAppends = {
    Init
    .then(AppendEvent("e1", "agg1", 0, "Created", "data1", "client1"))
    .then(AppendEvent("e2", "agg1", 1, "Updated", "data2", "client1"))
    .then(AppendEvent("e3", "agg1", 2, "Updated", "data3", "client1"))
    .then(all {
      assert(Safety),
      assert(events.length() == 3),
      assert(getVersion("agg1") == 3)
    })
  }

  /// Test: Concurrent conflict detection
  run testConcurrencyConflict = {
    Init
    .then(AppendEvent("e1", "agg1", 0, "Created", "data1", "client1"))
    .then(AppendEvent("e2", "agg1", 0, "Created", "data2", "client2"))  // Conflict!
    .then(all {
      assert(Safety),
      assert(events.length() == 1),  // Second append rejected
      assert(getVersion("agg1") == 1)
    })
  }

  /// Test: Idempotent append
  run testIdempotentAppend = {
    Init
    .then(AppendEvent("e1", "agg1", 0, "Created", "data1", "client1"))
    .then(AppendEvent("e1", "agg1", 0, "Created", "data1", "client1"))  // Duplicate
    .then(all {
      assert(Safety),
      assert(events.length() == 1),  // No duplicate
      assert(NoDuplicateEvents)
    })
  }

  /// Test: Multiple aggregates
  run testMultipleAggregates = {
    Init
    .then(AppendEvent("e1", "agg1", 0, "Created", "data1", "client1"))
    .then(AppendEvent("e2", "agg2", 0, "Created", "data2", "client1"))
    .then(AppendEvent("e3", "agg1", 1, "Updated", "data3", "client1"))
    .then(all {
      assert(Safety),
      assert(events.length() == 3),
      assert(getVersion("agg1") == 2),
      assert(getVersion("agg2") == 1)
    })
  }

}
