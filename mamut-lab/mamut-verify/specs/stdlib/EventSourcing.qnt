/// EventSourcing.qnt - Reusable operators for event sourcing verification
///
/// This module provides core operators for verifying event sourcing systems,
/// including aggregate ordering, optimistic concurrency, idempotency, and
/// snapshot correctness.
///
/// Event sourcing is a pattern where state changes are stored as a sequence
/// of events. This module formalizes the key properties that must hold for
/// correct event sourcing implementations.
module EventSourcing {

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// A generic event with required metadata
  type Event = {
    id: str,              // Unique event identifier
    aggregateId: str,     // Aggregate this event belongs to
    version: int,         // Version number within the aggregate
    timestamp: int,       // Logical timestamp for ordering
    eventType: str,       // Type discriminator for the event
    payload: str          // Serialized event data
  }

  /// A version entry tracking the current version of an aggregate
  type VersionEntry = {
    aggregateId: str,
    currentVersion: int,
    lastUpdated: int
  }

  /// A snapshot capturing aggregate state at a point in time
  type Snapshot[State] = {
    aggregateId: str,
    version: int,         // Version at which snapshot was taken
    state: State,         // The captured state
    timestamp: int
  }

  /// Result type for append operations
  type AppendResult =
    | Success(int)        // Returns new version
    | ConcurrencyConflict
    | DuplicateEvent

  // ============================================================================
  // Per-Aggregate Ordering Operators
  // ============================================================================

  /// Check that events are ordered by version within each aggregate.
  /// For any two events with the same aggregateId, if e1.version < e2.version,
  /// then e1 must appear before e2 in the sequence.
  ///
  /// Parameters:
  ///   events - Sequence of events to check
  ///   versionField - Function to extract version from event
  ///
  /// Returns: true if per-aggregate ordering holds
  pure def perAggregateOrdering(events: List[Event]): bool = {
    // Group events by aggregate
    val aggregateGroups = events.groupBy(e => e.aggregateId)

    // For each aggregate, check version ordering
    aggregateGroups.keys().forall(aggId => {
      val aggEvents = aggregateGroups.get(aggId)
      aggEvents.indices().forall(i => {
        aggEvents.indices().forall(j => {
          // If i < j in the list, then version at i should be <= version at j
          (i >= j) or (aggEvents[i].version <= aggEvents[j].version)
        })
      })
    })
  }

  /// Check strict version ordering: versions must be consecutive within aggregate
  pure def strictVersionOrdering(events: List[Event]): bool = {
    val aggregateGroups = events.groupBy(e => e.aggregateId)

    aggregateGroups.keys().forall(aggId => {
      val aggEvents = aggregateGroups.get(aggId)
      val sortedByVersion = aggEvents.sortBy(e => e.version)

      // Check that versions are consecutive starting from 1
      sortedByVersion.indices().forall(i => {
        sortedByVersion[i].version == i + 1
      })
    })
  }

  // ============================================================================
  // Optimistic Concurrency Operators
  // ============================================================================

  /// Validate optimistic concurrency control for a sequence of append operations.
  /// An append is valid if the expected version matches the current version.
  ///
  /// Parameters:
  ///   events - Sequence of events in the store
  ///   versions - Map from aggregateId to current version
  ///
  /// Returns: true if all events respect optimistic concurrency
  pure def optimisticConcurrencyValid(
    events: List[Event],
    versions: str -> int
  ): bool = {
    // Replay events and check each append would have succeeded
    val result = events.foldl(
      { valid: true, currentVersions: versions },
      (acc, event) => {
        val expectedVersion = acc.currentVersions.getOrElse(event.aggregateId, 0)
        val isValid = event.version == expectedVersion + 1
        {
          valid: acc.valid and isValid,
          currentVersions: acc.currentVersions.set(event.aggregateId, event.version)
        }
      }
    )
    result.valid
  }

  /// Check if an append operation would succeed given current versions
  pure def canAppend(
    aggregateId: str,
    expectedVersion: int,
    currentVersions: str -> int
  ): bool = {
    val actualVersion = currentVersions.getOrElse(aggregateId, 0)
    expectedVersion == actualVersion
  }

  /// Simulate an append and return the result
  pure def tryAppend(
    event: Event,
    currentVersions: str -> int,
    existingIds: Set[str]
  ): AppendResult = {
    if (existingIds.contains(event.id)) {
      DuplicateEvent
    } else {
      val actualVersion = currentVersions.getOrElse(event.aggregateId, 0)
      if (event.version != actualVersion + 1) {
        ConcurrencyConflict
      } else {
        Success(event.version)
      }
    }
  }

  // ============================================================================
  // Idempotency Operators
  // ============================================================================

  /// Check that all events have unique identifiers (idempotent appends).
  /// This ensures that replaying the same event multiple times has no effect.
  ///
  /// Parameters:
  ///   events - Sequence of events to check
  ///   idField - Function to extract unique ID from event
  ///
  /// Returns: true if all event IDs are unique
  pure def idempotentAppends(events: List[Event]): bool = {
    val ids = events.map(e => e.id)
    ids.toSet().size() == ids.length()
  }

  /// Check idempotency with a custom ID extraction function
  pure def idempotentAppendsBy(events: List[Event], idExtractor: Event => str): bool = {
    val ids = events.map(idExtractor)
    ids.toSet().size() == ids.length()
  }

  /// Given a set of known event IDs, check if an event is a duplicate
  pure def isDuplicate(event: Event, knownIds: Set[str]): bool = {
    knownIds.contains(event.id)
  }

  // ============================================================================
  // Snapshot Correctness Operators
  // ============================================================================

  /// Verify that a snapshot correctly represents the state after applying
  /// all events up to the snapshot version.
  ///
  /// Parameters:
  ///   snapshot - The snapshot to verify
  ///   events - All events for the aggregate
  ///   reducer - Function that applies an event to a state
  ///   initialState - Initial state before any events
  ///
  /// Returns: true if snapshot state matches reduced events
  pure def snapshotCorrectness[State](
    snapshot: Snapshot[State],
    events: List[Event],
    reducer: (State, Event) => State,
    initialState: State
  ): bool = {
    // Filter events for this aggregate up to snapshot version
    val relevantEvents = events
      .filter(e => e.aggregateId == snapshot.aggregateId)
      .filter(e => e.version <= snapshot.version)
      .sortBy(e => e.version)

    // Compute expected state by reducing events
    val computedState = relevantEvents.foldl(initialState, reducer)

    // Snapshot state must match computed state
    snapshot.state == computedState
  }

  /// Check that a snapshot is at a valid version point
  pure def snapshotVersionValid(
    snapshot: Snapshot[_],
    events: List[Event]
  ): bool = {
    val aggregateEvents = events.filter(e => e.aggregateId == snapshot.aggregateId)
    val maxVersion = if (aggregateEvents.length() == 0) {
      0
    } else {
      aggregateEvents.map(e => e.version).max()
    }
    snapshot.version >= 0 and snapshot.version <= maxVersion
  }

  // ============================================================================
  // Event Stream Operators
  // ============================================================================

  /// Get all events for a specific aggregate
  pure def eventsForAggregate(events: List[Event], aggregateId: str): List[Event] = {
    events.filter(e => e.aggregateId == aggregateId).sortBy(e => e.version)
  }

  /// Get events after a specific version for an aggregate
  pure def eventsAfterVersion(
    events: List[Event],
    aggregateId: str,
    afterVersion: int
  ): List[Event] = {
    events
      .filter(e => e.aggregateId == aggregateId)
      .filter(e => e.version > afterVersion)
      .sortBy(e => e.version)
  }

  /// Get the current version for an aggregate
  pure def currentVersion(events: List[Event], aggregateId: str): int = {
    val aggEvents = events.filter(e => e.aggregateId == aggregateId)
    if (aggEvents.length() == 0) {
      0
    } else {
      aggEvents.map(e => e.version).max()
    }
  }

  // ============================================================================
  // Causality and Ordering Operators
  // ============================================================================

  /// Check that timestamps are monotonically increasing within each aggregate
  pure def monotonicTimestamps(events: List[Event]): bool = {
    val aggregateGroups = events.groupBy(e => e.aggregateId)

    aggregateGroups.keys().forall(aggId => {
      val aggEvents = aggregateGroups.get(aggId).sortBy(e => e.version)
      aggEvents.indices().forall(i => {
        i == 0 or aggEvents[i - 1].timestamp <= aggEvents[i].timestamp
      })
    })
  }

  /// Check global timestamp ordering (for total ordering)
  pure def globalTimestampOrdering(events: List[Event]): bool = {
    events.indices().forall(i => {
      events.indices().forall(j => {
        (i >= j) or (events[i].timestamp <= events[j].timestamp)
      })
    })
  }

  // ============================================================================
  // Aggregate Integrity Operators
  // ============================================================================

  /// Check that no aggregate has gaps in its version sequence
  pure def noVersionGaps(events: List[Event]): bool = {
    val aggregateGroups = events.groupBy(e => e.aggregateId)

    aggregateGroups.keys().forall(aggId => {
      val versions = aggregateGroups.get(aggId).map(e => e.version).toSet()
      val maxVersion = versions.fold(0, (a, b) => if (a > b) a else b)

      // All versions from 1 to maxVersion must exist
      1.to(maxVersion).forall(v => versions.contains(v))
    })
  }

  /// Count events per aggregate
  pure def eventCountPerAggregate(events: List[Event]): str -> int = {
    events.foldl(
      Map(),
      (acc, e) => {
        val count = acc.getOrElse(e.aggregateId, 0)
        acc.set(e.aggregateId, count + 1)
      }
    )
  }

}
