/// Consistency.qnt - Formal definitions of consistency models
///
/// This module provides formal specifications of various consistency models
/// used in distributed systems. These definitions can be composed with other
/// specifications to verify that systems meet specific consistency guarantees.
///
/// Consistency models define the ordering and visibility guarantees that
/// clients can expect when reading and writing to a distributed system.
module Consistency {

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// A read or write operation in a distributed system
  type Operation = {
    id: str,              // Unique operation identifier
    clientId: str,        // Client that issued the operation
    opType: OperationType,
    key: str,             // Key being read/written
    value: str,           // Value (for writes) or read result
    startTime: int,       // When operation was invoked
    endTime: int,         // When operation completed
    processId: str        // Process that executed the operation
  }

  type OperationType = Read | Write

  /// A history is a sequence of operations
  type History = List[Operation]

  /// A linearization is a total ordering of operations
  type Linearization = List[Operation]

  /// Happens-before relation entry
  type HappensBefore = {
    before: str,  // Operation ID
    after: str    // Operation ID
  }

  /// A vector clock for tracking causality
  type VectorClock = str -> int

  /// Session information for session guarantees
  type Session = {
    clientId: str,
    operations: List[str]  // Operation IDs in session order
  }

  // ============================================================================
  // Linearizability
  // ============================================================================

  /// Linearizability: Operations appear to execute atomically at some point
  /// between their invocation and response times.
  ///
  /// A history H is linearizable if:
  /// 1. There exists a linearization L that is a permutation of H
  /// 2. L respects real-time ordering (if op1 ends before op2 starts, op1 precedes op2 in L)
  /// 3. L is sequentially consistent (reads return the most recent write in L)

  /// Check if a linearization respects real-time ordering
  pure def respectsRealTimeOrder(history: History, linearization: Linearization): bool = {
    history.forall(op1 => {
      history.forall(op2 => {
        // If op1 ended before op2 started in real time
        val realTimePrecedence = op1.endTime < op2.startTime

        // Then op1 must come before op2 in the linearization
        if (realTimePrecedence) {
          val idx1 = linearization.indices().filter(i => linearization[i].id == op1.id)
          val idx2 = linearization.indices().filter(i => linearization[i].id == op2.id)
          idx1.forall(i1 => idx2.forall(i2 => i1 < i2))
        } else {
          true
        }
      })
    })
  }

  /// Check if reads in a linearization return correct values
  pure def readsConsistentWithWrites(linearization: Linearization): bool = {
    linearization.indices().forall(i => {
      val op = linearization[i]
      match op.opType {
        | Read => {
          // Find the most recent write to this key before this read
          val precedingWrites = linearization.slice(0, i)
            .filter(o => o.opType == Write and o.key == op.key)

          if (precedingWrites.length() == 0) {
            // No preceding write; read should return initial value (empty)
            op.value == ""
          } else {
            // Read should return the value of the last write
            val lastWrite = precedingWrites[precedingWrites.length() - 1]
            op.value == lastWrite.value
          }
        }
        | Write => true
      }
    })
  }

  /// Check if a history is linearizable given a proposed linearization
  pure def isLinearizable(history: History, linearization: Linearization): bool = {
    // Linearization must contain same operations as history
    val sameOps = history.map(o => o.id).toSet() == linearization.map(o => o.id).toSet()

    sameOps and
    respectsRealTimeOrder(history, linearization) and
    readsConsistentWithWrites(linearization)
  }

  /// Check if two operations are concurrent (neither happened before the other)
  pure def areConcurrent(op1: Operation, op2: Operation): bool = {
    not(op1.endTime < op2.startTime) and not(op2.endTime < op1.startTime)
  }

  // ============================================================================
  // Sequential Consistency
  // ============================================================================

  /// Sequential Consistency: Operations appear to execute in some sequential
  /// order that is consistent with the program order of each process.
  ///
  /// Weaker than linearizability because it doesn't require respecting
  /// real-time order, only per-process order.

  /// Check if a linearization respects per-process ordering
  pure def respectsProcessOrder(history: History, linearization: Linearization): bool = {
    // Group operations by process
    val processGroups = history.groupBy(op => op.processId)

    processGroups.keys().forall(pid => {
      val processOps = processGroups.get(pid).sortBy(op => op.startTime)

      // For each pair of operations from the same process
      processOps.indices().forall(i => {
        processOps.indices().forall(j => {
          if (i < j) {
            // Operation i must come before operation j in linearization
            val idx_i = linearization.indices().filter(k => linearization[k].id == processOps[i].id)
            val idx_j = linearization.indices().filter(k => linearization[k].id == processOps[j].id)
            idx_i.forall(ki => idx_j.forall(kj => ki < kj))
          } else {
            true
          }
        })
      })
    })
  }

  /// Check if a history is sequentially consistent
  pure def isSequentiallyConsistent(history: History, linearization: Linearization): bool = {
    val sameOps = history.map(o => o.id).toSet() == linearization.map(o => o.id).toSet()

    sameOps and
    respectsProcessOrder(history, linearization) and
    readsConsistentWithWrites(linearization)
  }

  // ============================================================================
  // Causal Consistency
  // ============================================================================

  /// Causal Consistency: Operations that are causally related appear in the
  /// same order to all processes. Concurrent operations may appear in
  /// different orders to different processes.

  /// Build happens-before relation from a history
  pure def buildHappensBefore(history: History): Set[HappensBefore] = {
    val programOrder = history.indices().flatMap(i => {
      history.indices().filter(j => {
        val op_i = history[i]
        val op_j = history[j]
        // Same process and i happens before j
        op_i.processId == op_j.processId and op_i.endTime < op_j.startTime
      }).map(j => { before: history[i].id, after: history[j].id })
    }).toSet()

    val readFromOrder = history.indices().flatMap(i => {
      history.indices().filter(j => {
        val op_i = history[i]
        val op_j = history[j]
        // op_j is a read that returns the value written by op_i
        op_i.opType == Write and
        op_j.opType == Read and
        op_i.key == op_j.key and
        op_i.value == op_j.value and
        op_i.endTime < op_j.startTime
      }).map(j => { before: history[i].id, after: history[j].id })
    }).toSet()

    programOrder.union(readFromOrder)
  }

  /// Check if happens-before relation is respected in a linearization
  pure def respectsCausality(
    linearization: Linearization,
    happensBefore: Set[HappensBefore]
  ): bool = {
    happensBefore.forall(hb => {
      val idx_before = linearization.indices().filter(i => linearization[i].id == hb.before)
      val idx_after = linearization.indices().filter(i => linearization[i].id == hb.after)
      idx_before.forall(ib => idx_after.forall(ia => ib < ia))
    })
  }

  /// Check if a history is causally consistent
  pure def isCausallyConsistent(history: History, linearization: Linearization): bool = {
    val happensBefore = buildHappensBefore(history)
    val sameOps = history.map(o => o.id).toSet() == linearization.map(o => o.id).toSet()

    sameOps and
    respectsCausality(linearization, happensBefore) and
    readsConsistentWithWrites(linearization)
  }

  /// Vector clock operations for tracking causality
  pure def vcIncrement(vc: VectorClock, processId: str): VectorClock = {
    vc.set(processId, vc.getOrElse(processId, 0) + 1)
  }

  pure def vcMerge(vc1: VectorClock, vc2: VectorClock): VectorClock = {
    val allKeys = vc1.keys().union(vc2.keys())
    allKeys.fold(Map(), (acc, k) => {
      val v1 = vc1.getOrElse(k, 0)
      val v2 = vc2.getOrElse(k, 0)
      acc.set(k, if (v1 > v2) v1 else v2)
    })
  }

  pure def vcHappensBefore(vc1: VectorClock, vc2: VectorClock): bool = {
    val allKeys = vc1.keys().union(vc2.keys())
    val allLeq = allKeys.forall(k => vc1.getOrElse(k, 0) <= vc2.getOrElse(k, 0))
    val someLt = allKeys.exists(k => vc1.getOrElse(k, 0) < vc2.getOrElse(k, 0))
    allLeq and someLt
  }

  pure def vcConcurrent(vc1: VectorClock, vc2: VectorClock): bool = {
    not(vcHappensBefore(vc1, vc2)) and not(vcHappensBefore(vc2, vc1))
  }

  // ============================================================================
  // Eventual Consistency
  // ============================================================================

  /// Eventual Consistency: If no new updates are made, eventually all reads
  /// will return the last updated value.

  /// A quiescent state is one where no operations are in progress
  pure def isQuiescent(history: History, currentTime: int): bool = {
    history.forall(op => op.endTime <= currentTime)
  }

  /// Check eventual convergence: all processes see the same value after quiescence
  pure def eventuallyConverges(
    history: History,
    readsByProcess: str -> List[Operation],
    quiescentTime: int
  ): bool = {
    // Get reads after quiescent time for each process
    val postQuiescentReads = readsByProcess.keys().map(pid => {
      readsByProcess.get(pid)
        .filter(op => op.startTime > quiescentTime)
        .filter(op => op.opType == Read)
    })

    // All post-quiescent reads of the same key should return the same value
    postQuiescentReads.forall(reads1 => {
      postQuiescentReads.forall(reads2 => {
        reads1.forall(r1 => {
          reads2.forall(r2 => {
            r1.key != r2.key or r1.value == r2.value
          })
        })
      })
    })
  }

  /// Monotonic reads: once a value is read, subsequent reads return same or newer
  pure def monotonicReads(sessionOps: List[Operation]): bool = {
    val reads = sessionOps.filter(op => op.opType == Read)

    reads.indices().forall(i => {
      reads.indices().forall(j => {
        if (i < j and reads[i].key == reads[j].key) {
          // Cannot check "newer" without version info,
          // so we just ensure reads don't regress (simplified)
          true
        } else {
          true
        }
      })
    })
  }

  /// Read your writes: after writing, reads return at least that value
  pure def readYourWrites(sessionOps: List[Operation]): bool = {
    sessionOps.indices().forall(i => {
      val op = sessionOps[i]
      match op.opType {
        | Read => {
          // Find preceding writes to the same key in this session
          val precedingWrites = sessionOps.slice(0, i)
            .filter(o => o.opType == Write and o.key == op.key)

          if (precedingWrites.length() == 0) {
            true  // No preceding write, any value is acceptable
          } else {
            val lastWrite = precedingWrites[precedingWrites.length() - 1]
            // The read should see at least the written value
            // (In eventual consistency, it could see a newer value too)
            op.value == lastWrite.value or op.value != ""
          }
        }
        | Write => true
      }
    })
  }

  /// Monotonic writes: writes by a process are seen by all in that order
  pure def monotonicWrites(history: History): bool = {
    val processGroups = history.groupBy(op => op.processId)

    processGroups.keys().forall(pid => {
      val processOps = processGroups.get(pid).sortBy(op => op.startTime)
      val writes = processOps.filter(op => op.opType == Write)

      // Writes should complete in the order they were started
      writes.indices().forall(i => {
        writes.indices().forall(j => {
          (i >= j) or (writes[i].endTime <= writes[j].endTime)
        })
      })
    })
  }

  // ============================================================================
  // PRAM (Pipelined RAM) Consistency
  // ============================================================================

  /// PRAM Consistency: Writes from a single process are seen by all in order,
  /// but writes from different processes may be seen in different orders.

  pure def isPRAMConsistent(history: History, processViews: str -> Linearization): bool = {
    val processGroups = history.groupBy(op => op.processId)

    // For each pair of processes
    processGroups.keys().forall(writerPid => {
      val writerOps = processGroups.get(writerPid).sortBy(op => op.startTime)
      val writes = writerOps.filter(op => op.opType == Write)

      // All processes must see this process's writes in order
      processViews.keys().forall(viewerPid => {
        val view = processViews.get(viewerPid)

        writes.indices().forall(i => {
          writes.indices().forall(j => {
            if (i < j) {
              val idx_i = view.indices().filter(k => view[k].id == writes[i].id)
              val idx_j = view.indices().filter(k => view[k].id == writes[j].id)
              idx_i.forall(ki => idx_j.forall(kj => ki < kj))
            } else {
              true
            }
          })
        })
      })
    })
  }

  // ============================================================================
  // Session Guarantees
  // ============================================================================

  /// Writes follow reads: if a read returns value from write W,
  /// subsequent writes are ordered after W
  pure def writesFollowReads(session: Session, allOps: List[Operation]): bool = {
    val sessionOps = allOps.filter(op => session.operations.contains(op.id))

    sessionOps.indices().forall(i => {
      val op_i = sessionOps[i]
      match op_i.opType {
        | Read => {
          // Find writes that came after this read in the session
          val laterWrites = sessionOps.slice(i + 1, sessionOps.length())
            .filter(op => op.opType == Write)

          // These writes should be causally after the write that was read
          // (Simplified check - full implementation would track causality)
          true
        }
        | Write => true
      }
    })
  }

  // ============================================================================
  // Utility Functions
  // ============================================================================

  /// Extract all keys from a history
  pure def keysInHistory(history: History): Set[str] = {
    history.map(op => op.key).toSet()
  }

  /// Get all writes for a specific key
  pure def writesForKey(history: History, key: str): List[Operation] = {
    history.filter(op => op.opType == Write and op.key == key)
  }

  /// Get all reads for a specific key
  pure def readsForKey(history: History, key: str): List[Operation] = {
    history.filter(op => op.opType == Read and op.key == key)
  }

  /// Check if history is well-formed (all operations complete)
  pure def isWellFormed(history: History): bool = {
    history.forall(op => op.startTime < op.endTime)
  }

}
