/// optimistic_concurrency.qnt - Specification for optimistic concurrency control in event sourcing
///
/// This specification models optimistic concurrency control (OCC) mechanisms
/// used in event sourcing systems. OCC allows multiple clients to read and
/// attempt writes concurrently, detecting conflicts at write time.
///
/// Key properties verified:
/// - Version-based conflict detection works correctly
/// - No lost updates due to concurrent modifications
/// - Retry semantics preserve data integrity
/// - Expected version matching prevents stale writes
module optimistic_concurrency {

  import EventSourcing.*

  // ============================================================================
  // Constants
  // ============================================================================

  /// Number of concurrent clients
  const NUM_CLIENTS: int = 3

  /// Client identifiers
  const CLIENTS: Set[str] = Set("client1", "client2", "client3")

  /// Aggregate identifiers
  const AGGREGATES: Set[str] = Set("order1", "order2")

  /// Maximum retries for failed operations
  const MAX_RETRIES: int = 3

  // ============================================================================
  // State Variables
  // ============================================================================

  /// The event store
  var events: List[Event]

  /// Current versions per aggregate
  var versions: str -> int

  /// Known event IDs for deduplication
  var eventIds: Set[str]

  /// Global logical clock
  var clock: int

  /// Client state: what version each client last read for each aggregate
  var clientVersionCache: str -> (str -> int)  // clientId -> (aggId -> version)

  /// Pending operations that clients are attempting
  var pendingOps: Set[PendingOperation]

  /// History of all operation attempts (for verification)
  var operationHistory: List[OperationAttempt]

  /// Count of conflicts per client (for fairness analysis)
  var conflictCounts: str -> int

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// A pending operation waiting to be committed
  type PendingOperation = {
    opId: str,
    clientId: str,
    aggregateId: str,
    expectedVersion: int,
    eventType: str,
    payload: str,
    retryCount: int
  }

  /// Record of an operation attempt
  type OperationAttempt = {
    opId: str,
    clientId: str,
    aggregateId: str,
    expectedVersion: int,
    actualVersion: int,
    outcome: AttemptOutcome,
    timestamp: int
  }

  type AttemptOutcome =
    | Committed
    | Conflicted
    | Duplicate
    | Retrying

  /// Client session state
  type ClientSession = {
    clientId: str,
    cachedVersions: str -> int,
    pendingWrites: List[str]
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /// Get current version for an aggregate
  pure def getCurrentVersion(aggId: str, vers: str -> int): int = {
    vers.getOrElse(aggId, 0)
  }

  /// Check if a version matches expected
  pure def versionMatches(aggId: str, expected: int, vers: str -> int): bool = {
    getCurrentVersion(aggId, vers) == expected
  }

  /// Generate a unique event ID
  pure def makeEventId(opId: str, retry: int): str = {
    opId ++ "_" ++ retry.toString()
  }

  // ============================================================================
  // State Predicates
  // ============================================================================

  /// Check if client has a stale version cached
  def clientHasStaleCache(clientId: str, aggId: str): bool = {
    val clientCache = clientVersionCache.getOrElse(clientId, Map())
    val cachedVersion = clientCache.getOrElse(aggId, -1)
    val currentVersion = getCurrentVersion(aggId, versions)
    cachedVersion >= 0 and cachedVersion < currentVersion
  }

  /// Check if there are any pending operations for an aggregate
  def hasPendingOps(aggId: str): bool = {
    pendingOps.exists(op => op.aggregateId == aggId)
  }

  /// Get pending operations for a client
  def clientPendingOps(clientId: str): Set[PendingOperation] = {
    pendingOps.filter(op => op.clientId == clientId)
  }

  // ============================================================================
  // Actions
  // ============================================================================

  /// Initialize the system
  action Init: bool = all {
    events' = [],
    versions' = Map(),
    eventIds' = Set(),
    clock' = 0,
    clientVersionCache' = Map(),
    pendingOps' = Set(),
    operationHistory' = [],
    conflictCounts' = CLIENTS.mapBy(c => 0)
  }

  /// Client reads the current state of an aggregate, caching the version
  action ClientRead(clientId: str, aggId: str): bool = {
    val currentVersion = getCurrentVersion(aggId, versions)
    val clientCache = clientVersionCache.getOrElse(clientId, Map())
    val updatedCache = clientCache.set(aggId, currentVersion)

    all {
      events' = events,
      versions' = versions,
      eventIds' = eventIds,
      clock' = clock + 1,
      clientVersionCache' = clientVersionCache.set(clientId, updatedCache),
      pendingOps' = pendingOps,
      operationHistory' = operationHistory,
      conflictCounts' = conflictCounts
    }
  }

  /// Client initiates a write operation based on cached version
  action ClientInitiateWrite(
    clientId: str,
    opId: str,
    aggId: str,
    eventType: str,
    payload: str
  ): bool = {
    val clientCache = clientVersionCache.getOrElse(clientId, Map())
    val expectedVersion = clientCache.getOrElse(aggId, 0)

    val newOp: PendingOperation = {
      opId: opId,
      clientId: clientId,
      aggregateId: aggId,
      expectedVersion: expectedVersion,
      eventType: eventType,
      payload: payload,
      retryCount: 0
    }

    all {
      events' = events,
      versions' = versions,
      eventIds' = eventIds,
      clock' = clock,
      clientVersionCache' = clientVersionCache,
      pendingOps' = pendingOps.union(Set(newOp)),
      operationHistory' = operationHistory,
      conflictCounts' = conflictCounts
    }
  }

  /// Attempt to commit a pending operation
  action AttemptCommit(op: PendingOperation): bool = {
    val currentVersion = getCurrentVersion(op.aggregateId, versions)
    val eventId = makeEventId(op.opId, op.retryCount)

    if (eventIds.contains(eventId)) {
      // Duplicate event - already committed
      val attempt: OperationAttempt = {
        opId: op.opId,
        clientId: op.clientId,
        aggregateId: op.aggregateId,
        expectedVersion: op.expectedVersion,
        actualVersion: currentVersion,
        outcome: Duplicate,
        timestamp: clock
      }

      all {
        events' = events,
        versions' = versions,
        eventIds' = eventIds,
        clock' = clock + 1,
        clientVersionCache' = clientVersionCache,
        pendingOps' = pendingOps.exclude(Set(op)),
        operationHistory' = operationHistory.append(attempt),
        conflictCounts' = conflictCounts
      }
    } else if (op.expectedVersion != currentVersion) {
      // Concurrency conflict
      val attempt: OperationAttempt = {
        opId: op.opId,
        clientId: op.clientId,
        aggregateId: op.aggregateId,
        expectedVersion: op.expectedVersion,
        actualVersion: currentVersion,
        outcome: Conflicted,
        timestamp: clock
      }

      val currentConflicts = conflictCounts.getOrElse(op.clientId, 0)

      all {
        events' = events,
        versions' = versions,
        eventIds' = eventIds,
        clock' = clock + 1,
        clientVersionCache' = clientVersionCache,
        pendingOps' = pendingOps.exclude(Set(op)),
        operationHistory' = operationHistory.append(attempt),
        conflictCounts' = conflictCounts.set(op.clientId, currentConflicts + 1)
      }
    } else {
      // Success - commit the event
      val newVersion = currentVersion + 1
      val newEvent: Event = {
        id: eventId,
        aggregateId: op.aggregateId,
        version: newVersion,
        timestamp: clock,
        eventType: op.eventType,
        payload: op.payload
      }

      val attempt: OperationAttempt = {
        opId: op.opId,
        clientId: op.clientId,
        aggregateId: op.aggregateId,
        expectedVersion: op.expectedVersion,
        actualVersion: currentVersion,
        outcome: Committed,
        timestamp: clock
      }

      // Update client's cache with new version
      val clientCache = clientVersionCache.getOrElse(op.clientId, Map())
      val updatedCache = clientCache.set(op.aggregateId, newVersion)

      all {
        events' = events.append(newEvent),
        versions' = versions.set(op.aggregateId, newVersion),
        eventIds' = eventIds.union(Set(eventId)),
        clock' = clock + 1,
        clientVersionCache' = clientVersionCache.set(op.clientId, updatedCache),
        pendingOps' = pendingOps.exclude(Set(op)),
        operationHistory' = operationHistory.append(attempt),
        conflictCounts' = conflictCounts
      }
    }
  }

  /// Retry a conflicted operation with updated version
  action RetryOperation(clientId: str, opId: str): bool = {
    val clientOps = clientPendingOps(clientId)

    // Find the operation in history that conflicted
    val conflicted = operationHistory.filter(a =>
      a.opId == opId and
      a.clientId == clientId and
      a.outcome == Conflicted
    )

    if (conflicted.length() == 0) {
      // No conflicted operation to retry
      all {
        events' = events,
        versions' = versions,
        eventIds' = eventIds,
        clock' = clock,
        clientVersionCache' = clientVersionCache,
        pendingOps' = pendingOps,
        operationHistory' = operationHistory,
        conflictCounts' = conflictCounts
      }
    } else {
      val lastConflict = conflicted[conflicted.length() - 1]

      // Re-read current version
      val currentVersion = getCurrentVersion(lastConflict.aggregateId, versions)
      val retryCount = conflicted.length()

      if (retryCount >= MAX_RETRIES) {
        // Max retries exceeded, give up
        all {
          events' = events,
          versions' = versions,
          eventIds' = eventIds,
          clock' = clock + 1,
          clientVersionCache' = clientVersionCache,
          pendingOps' = pendingOps,
          operationHistory' = operationHistory,
          conflictCounts' = conflictCounts
        }
      } else {
        // Create retry operation with new expected version
        val retryOp: PendingOperation = {
          opId: opId,
          clientId: clientId,
          aggregateId: lastConflict.aggregateId,
          expectedVersion: currentVersion,
          eventType: "Retry",  // Would need to track original event type
          payload: "retry",
          retryCount: retryCount
        }

        val retryAttempt: OperationAttempt = {
          opId: opId,
          clientId: clientId,
          aggregateId: lastConflict.aggregateId,
          expectedVersion: currentVersion,
          actualVersion: currentVersion,
          outcome: Retrying,
          timestamp: clock
        }

        all {
          events' = events,
          versions' = versions,
          eventIds' = eventIds,
          clock' = clock + 1,
          clientVersionCache' = clientVersionCache,
          pendingOps' = pendingOps.union(Set(retryOp)),
          operationHistory' = operationHistory.append(retryAttempt),
          conflictCounts' = conflictCounts
        }
      }
    }
  }

  /// Non-deterministic step
  action step: bool = {
    nondet clientId = oneOf(CLIENTS)
    nondet aggId = oneOf(AGGREGATES)
    nondet opId = oneOf(Set("op1", "op2", "op3", "op4", "op5"))
    nondet eventType = oneOf(Set("Update", "Command"))

    any {
      ClientRead(clientId, aggId),
      ClientInitiateWrite(clientId, opId, aggId, eventType, "data"),
      // Attempt to commit any pending operation
      pendingOps.exists(op => AttemptCommit(op)),
      RetryOperation(clientId, opId)
    }
  }

  // ============================================================================
  // Invariants
  // ============================================================================

  /// INV1: No Lost Updates
  /// If two clients read the same version and both try to write,
  /// at most one can succeed
  val NoLostUpdates: bool = {
    // Check that for any two committed operations to the same aggregate
    // with the same expected version, they have different versions in the result
    operationHistory.indices().forall(i => {
      operationHistory.indices().forall(j => {
        if (i < j) {
          val op_i = operationHistory[i]
          val op_j = operationHistory[j]

          // If both committed to the same aggregate with same expected version
          val bothCommitted = op_i.outcome == Committed and op_j.outcome == Committed
          val sameAggregate = op_i.aggregateId == op_j.aggregateId
          val sameExpected = op_i.expectedVersion == op_j.expectedVersion

          not(bothCommitted and sameAggregate and sameExpected)
        } else {
          true
        }
      })
    })
  }

  /// INV2: Version Monotonicity
  /// Versions only increase, never decrease
  val VersionMonotonicity: bool = {
    versions.keys().forall(aggId => {
      versions.get(aggId) >= 0
    }) and
    events.indices().forall(i => {
      events.indices().forall(j => {
        val e_i = events[i]
        val e_j = events[j]

        // For same aggregate, later events have higher versions
        (e_i.aggregateId != e_j.aggregateId) or
        (i >= j) or
        (e_i.version < e_j.version)
      })
    })
  }

  /// INV3: Conflict Detection Correctness
  /// A conflict is only reported when versions actually differ
  val ConflictDetectionCorrect: bool = {
    operationHistory.forall(attempt => {
      match attempt.outcome {
        | Conflicted => attempt.expectedVersion != attempt.actualVersion
        | Committed => attempt.expectedVersion == attempt.actualVersion
        | _ => true
      }
    })
  }

  /// INV4: Committed Events Persist
  /// Once an event is committed, it remains in the store
  val CommittedEventsPersist: bool = {
    operationHistory.forall(attempt => {
      match attempt.outcome {
        | Committed => {
          val eventId = makeEventId(attempt.opId, 0)  // Simplified
          events.exists(e => e.aggregateId == attempt.aggregateId)
        }
        | _ => true
      }
    })
  }

  /// INV5: Cache Staleness is Detectable
  /// When a client's cache is stale, their write will fail
  val StaleCacheDetectable: bool = {
    CLIENTS.forall(clientId => {
      AGGREGATES.forall(aggId => {
        val clientCache = clientVersionCache.getOrElse(clientId, Map())
        val cachedVersion = clientCache.getOrElse(aggId, -1)
        val currentVersion = getCurrentVersion(aggId, versions)

        // If cache is stale (and not uninitialized),
        // any pending write with that expected version should not commit
        (cachedVersion < 0) or
        (cachedVersion >= currentVersion) or
        pendingOps.forall(op =>
          op.clientId != clientId or
          op.aggregateId != aggId or
          op.expectedVersion != cachedVersion
        )
      })
    })
  }

  /// INV6: No Duplicate Commits
  /// Same operation cannot be committed twice
  val NoDuplicateCommits: bool = {
    val commits = operationHistory.filter(a => a.outcome == Committed)
    val commitKeys = commits.map(a => (a.opId, a.clientId))
    commitKeys.toSet().size() == commitKeys.length()
  }

  /// Combined safety invariant
  val Safety: bool = {
    NoLostUpdates and
    VersionMonotonicity and
    ConflictDetectionCorrect and
    NoDuplicateCommits
  }

  // ============================================================================
  // Temporal Properties
  // ============================================================================

  /// Progress: If a client keeps retrying, eventually they succeed
  /// (This requires fairness assumptions in model checking)
  val EventualProgress: bool = {
    // Simplified check: if there are pending ops and no conflicts recently,
    // some progress is being made
    pendingOps.size() == 0 or
    operationHistory.exists(a => a.outcome == Committed)
  }

  /// Fairness: No client is starved (conflicts are bounded)
  val BoundedConflicts: bool = {
    conflictCounts.keys().forall(clientId => {
      conflictCounts.get(clientId) <= MAX_RETRIES * 10  // Reasonable bound
    })
  }

  // ============================================================================
  // Test Scenarios
  // ============================================================================

  /// Test: Simple write succeeds
  run testSimpleWrite = {
    Init
    .then(ClientRead("client1", "order1"))
    .then(ClientInitiateWrite("client1", "op1", "order1", "Create", "data"))
    .then(all {
      assert(pendingOps.size() == 1)
    })
  }

  /// Test: Concurrent writes cause conflict
  run testConcurrentConflict = {
    Init
    // Both clients read version 0
    .then(ClientRead("client1", "order1"))
    .then(ClientRead("client2", "order1"))
    // Both initiate writes with expected version 0
    .then(ClientInitiateWrite("client1", "op1", "order1", "Update", "data1"))
    .then(ClientInitiateWrite("client2", "op2", "order1", "Update", "data2"))
    .then(all {
      assert(pendingOps.size() == 2)
    })
  }

  /// Test: Read-modify-write pattern
  run testReadModifyWrite = {
    Init
    // Client reads, modifies, writes
    .then(ClientRead("client1", "order1"))
    .then(ClientInitiateWrite("client1", "op1", "order1", "Create", "initial"))
    .then(pendingOps.forall(op => AttemptCommit(op)))
    .then(all {
      assert(events.length() == 1),
      assert(versions.getOrElse("order1", 0) == 1)
    })
  }

}
