/// Compliance.qnt - Reusable operators for compliance verification
///
/// This module provides core operators for verifying compliance systems,
/// including decision determinism, temporal accuracy, auditability, and
/// policy versioning correctness.
///
/// Compliance systems must demonstrate that decisions are made consistently,
/// at the correct time, with the correct policy version, and can be
/// reconstructed for audit purposes.
module Compliance {

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// An input to a compliance decision
  type ComplianceInput = {
    id: str,              // Unique input identifier
    entityId: str,        // Entity being evaluated (customer, transaction, etc.)
    inputData: str,       // Serialized input data
    timestamp: int,       // When the input was received
    context: str          // Additional context (jurisdiction, channel, etc.)
  }

  /// A compliance decision
  type ComplianceDecision = {
    id: str,              // Unique decision identifier
    inputId: str,         // Reference to the input that triggered this decision
    outcome: DecisionOutcome,
    ruleId: str,          // Rule that produced this decision
    policyVersion: str,   // Version of the policy used
    effectiveDate: int,   // When the policy became effective
    decisionTime: int,    // When the decision was made
    explanation: str      // Human-readable explanation
  }

  type DecisionOutcome =
    | Approved
    | Denied
    | RequiresReview
    | Deferred

  /// A compliance rule within a policy
  type ComplianceRule = {
    id: str,              // Rule identifier
    policyId: str,        // Parent policy
    condition: str,       // Rule condition (serialized)
    outcome: DecisionOutcome,
    priority: int,        // Rule priority for conflict resolution
    effectiveFrom: int,   // When rule becomes effective
    effectiveTo: int      // When rule expires (-1 for no expiry)
  }

  /// A compliance policy
  type CompliancePolicy = {
    id: str,              // Policy identifier
    version: str,         // Version string
    effectiveFrom: int,   // Effective date
    effectiveTo: int,     // Expiry date (-1 for no expiry)
    rules: Set[str]       // Rule IDs in this policy
  }

  /// An audit log entry
  type AuditLogEntry = {
    id: str,              // Log entry identifier
    decisionId: str,      // Decision this entry records
    inputSnapshot: str,   // Snapshot of input at decision time
    policySnapshot: str,  // Snapshot of policy version used
    ruleTrace: List[str], // Ordered list of rules evaluated
    timestamp: int        // When this was logged
  }

  /// An evaluator function representation
  type Evaluator = {
    id: str,              // Evaluator identifier
    version: str,         // Evaluator version
    algorithm: str        // Algorithm identifier
  }

  /// A clock abstraction for temporal reasoning
  type ComplianceClock = {
    currentTime: int,     // Current logical time
    timezone: str,        // Timezone for date calculations
    precision: int        // Time precision in milliseconds
  }

  // ============================================================================
  // Determinism Operators
  // ============================================================================

  /// Verify that the same inputs produce the same decisions when evaluated
  /// by the same evaluator under the same conditions.
  ///
  /// Determinism is crucial for compliance systems because:
  /// 1. Auditors must be able to reproduce decisions
  /// 2. Customers should receive consistent treatment
  /// 3. Legal challenges require demonstrable consistency
  ///
  /// Parameters:
  ///   inputs - Set of compliance inputs
  ///   decisions - List of decisions made
  ///   evaluator - The evaluator used
  ///
  /// Returns: true if all identical inputs produced identical decisions
  pure def deterministic(
    inputs: Set[ComplianceInput],
    decisions: List[ComplianceDecision],
    evaluator: Evaluator
  ): bool = {
    // Group decisions by their input characteristics
    val inputGroups = decisions.groupBy(d => d.inputId)

    // For each input, all decisions should have the same outcome
    // when made with the same policy version
    inputs.forall(input => {
      val inputDecisions = decisions.filter(d => d.inputId == input.id)

      // Group by policy version
      val byPolicyVersion = inputDecisions.groupBy(d => d.policyVersion)

      // Within each policy version, all outcomes must match
      byPolicyVersion.keys().forall(pv => {
        val versioned = byPolicyVersion.get(pv)
        versioned.length() <= 1 or
        versioned.forall(d => d.outcome == versioned[0].outcome)
      })
    })
  }

  /// Verify determinism for a specific input-output pair
  pure def deterministicPair(
    input1: ComplianceInput,
    decision1: ComplianceDecision,
    input2: ComplianceInput,
    decision2: ComplianceDecision
  ): bool = {
    // If inputs are equivalent, decisions must be equivalent
    val inputsEquivalent =
      input1.inputData == input2.inputData and
      input1.context == input2.context

    val decisionsOnSamePolicy =
      decision1.policyVersion == decision2.policyVersion

    not(inputsEquivalent and decisionsOnSamePolicy) or
    decision1.outcome == decision2.outcome
  }

  /// Check that no external side effects influence decision outcomes
  pure def noSideEffects(
    decisions: List[ComplianceDecision],
    externalStateChanges: List[int]  // Timestamps of external changes
  ): bool = {
    // Decisions should not correlate with external state changes
    // (Simplified check: decision outcomes are stable across time windows)
    decisions.indices().forall(i => {
      decisions.indices().forall(j => {
        if (i < j) {
          val d1 = decisions[i]
          val d2 = decisions[j]

          // Same input should produce same output regardless of when evaluated
          d1.inputId != d2.inputId or
          d1.policyVersion != d2.policyVersion or
          d1.outcome == d2.outcome
        } else {
          true
        }
      })
    })
  }

  // ============================================================================
  // Temporal Accuracy Operators
  // ============================================================================

  /// Verify that decisions use the correct policy version for their timestamp.
  ///
  /// Temporal accuracy ensures that:
  /// 1. Decisions are made using the policy effective at the decision time
  /// 2. Retroactive policy changes don't affect past decisions
  /// 3. Future-dated policies don't apply prematurely
  ///
  /// Parameters:
  ///   decisions - List of compliance decisions
  ///   policies - List of policies with effective dates
  ///   clock - The clock used for time calculations
  ///
  /// Returns: true if all decisions use temporally correct policy versions
  pure def temporallyAccurate(
    decisions: List[ComplianceDecision],
    policies: List[CompliancePolicy],
    clock: ComplianceClock
  ): bool = {
    decisions.forall(decision => {
      // Find the policy version used
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      if (usedPolicy.length() == 0) {
        false  // Unknown policy version
      } else {
        val policy = usedPolicy[0]

        // Decision time must be within policy's effective window
        val afterEffective = decision.decisionTime >= policy.effectiveFrom
        val beforeExpiry = policy.effectiveTo == -1 or
                           decision.decisionTime <= policy.effectiveTo

        afterEffective and beforeExpiry
      }
    })
  }

  /// Verify that the effective date in the decision matches the policy
  pure def effectiveDateCorrect(
    decision: ComplianceDecision,
    policies: List[CompliancePolicy]
  ): bool = {
    val matchingPolicies = policies.filter(p => p.version == decision.policyVersion)

    matchingPolicies.length() > 0 and
    matchingPolicies.forall(p => decision.effectiveDate == p.effectiveFrom)
  }

  /// Check that no decision was made with a future-dated policy
  pure def noFutureDatedPolicies(
    decisions: List[ComplianceDecision],
    policies: List[CompliancePolicy],
    currentTime: int
  ): bool = {
    decisions.forall(decision => {
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      usedPolicy.length() == 0 or
      usedPolicy.forall(p => p.effectiveFrom <= decision.decisionTime)
    })
  }

  /// Verify time window constraints for decisions
  pure def withinTimeWindow(
    decision: ComplianceDecision,
    windowStart: int,
    windowEnd: int
  ): bool = {
    decision.decisionTime >= windowStart and
    decision.decisionTime <= windowEnd
  }

  // ============================================================================
  // Reconstructability Operators
  // ============================================================================

  /// Verify that all decisions can be reconstructed from the audit log.
  ///
  /// Reconstructability is required for:
  /// 1. Regulatory audits and examinations
  /// 2. Dispute resolution
  /// 3. Post-hoc analysis and compliance reporting
  ///
  /// Parameters:
  ///   decisions - List of compliance decisions
  ///   auditLog - List of audit log entries
  ///
  /// Returns: true if all decisions have corresponding audit entries
  pure def reconstructable(
    decisions: List[ComplianceDecision],
    auditLog: List[AuditLogEntry]
  ): bool = {
    // Every decision must have a corresponding audit entry
    decisions.forall(decision => {
      auditLog.exists(entry => {
        entry.decisionId == decision.id
      })
    })
  }

  /// Verify that audit log entries contain sufficient information
  pure def auditLogComplete(
    auditLog: List[AuditLogEntry]
  ): bool = {
    auditLog.forall(entry => {
      entry.inputSnapshot != "" and
      entry.policySnapshot != "" and
      entry.ruleTrace.length() > 0
    })
  }

  /// Verify audit log immutability (no entries are modified or deleted)
  pure def auditLogImmutable(
    previousLog: List[AuditLogEntry],
    currentLog: List[AuditLogEntry]
  ): bool = {
    // Previous entries must still exist in current log unchanged
    previousLog.forall(prevEntry => {
      currentLog.exists(currEntry => {
        currEntry.id == prevEntry.id and
        currEntry.decisionId == prevEntry.decisionId and
        currEntry.inputSnapshot == prevEntry.inputSnapshot and
        currEntry.policySnapshot == prevEntry.policySnapshot and
        currEntry.timestamp == prevEntry.timestamp
      })
    })
  }

  /// Verify that the rule trace in audit log matches the decision
  pure def ruleTraceConsistent(
    decision: ComplianceDecision,
    auditEntry: AuditLogEntry,
    rules: List[ComplianceRule]
  ): bool = {
    // The rule that produced the decision must be in the trace
    auditEntry.ruleTrace.contains(decision.ruleId) and
    // All rules in the trace must exist
    auditEntry.ruleTrace.forall(ruleId => {
      rules.exists(r => r.id == ruleId)
    })
  }

  // ============================================================================
  // Policy Version Correctness Operators
  // ============================================================================

  /// Verify that decisions reference valid policy versions.
  ///
  /// Policy versioning correctness ensures:
  /// 1. Decisions reference policies that actually exist
  /// 2. Policy versions are applied in the correct order
  /// 3. No gaps or overlaps in policy effective dates
  ///
  /// Parameters:
  ///   decisions - List of compliance decisions
  ///   policies - List of policies
  ///
  /// Returns: true if all decisions reference valid policy versions
  pure def policyVersionCorrect(
    decisions: List[ComplianceDecision],
    policies: List[CompliancePolicy]
  ): bool = {
    val policyVersions = policies.map(p => p.version).toSet()

    decisions.forall(decision => {
      policyVersions.contains(decision.policyVersion)
    })
  }

  /// Verify that policy versions don't have overlapping effective periods
  pure def noOverlappingPolicies(
    policies: List[CompliancePolicy]
  ): bool = {
    // Group by policy ID to check versions of the same policy
    val byPolicyId = policies.groupBy(p => p.id)

    byPolicyId.keys().forall(policyId => {
      val versions = byPolicyId.get(policyId)

      versions.indices().forall(i => {
        versions.indices().forall(j => {
          if (i < j) {
            val p1 = versions[i]
            val p2 = versions[j]

            // No overlap: p1 ends before p2 starts, or p2 ends before p1 starts
            val p1EndTime = if (p1.effectiveTo == -1) 2147483647 else p1.effectiveTo
            val p2EndTime = if (p2.effectiveTo == -1) 2147483647 else p2.effectiveTo

            p1EndTime < p2.effectiveFrom or p2EndTime < p1.effectiveFrom
          } else {
            true
          }
        })
      })
    })
  }

  /// Verify that there are no gaps in policy coverage
  pure def continuousPolicyCoverage(
    policies: List[CompliancePolicy],
    coverageStart: int,
    coverageEnd: int
  ): bool = {
    // Sort policies by effective date
    val sorted = policies.sortBy(p => p.effectiveFrom)

    if (sorted.length() == 0) {
      false  // No policies means no coverage
    } else {
      // First policy must start at or before coverage start
      val startsEarly = sorted[0].effectiveFrom <= coverageStart

      // Check for gaps between consecutive policies
      val noGaps = sorted.indices().forall(i => {
        if (i == sorted.length() - 1) {
          // Last policy must extend to or past coverage end
          sorted[i].effectiveTo == -1 or sorted[i].effectiveTo >= coverageEnd
        } else {
          // Current policy must end when or after next begins
          val currentEnd = if (sorted[i].effectiveTo == -1) 2147483647 else sorted[i].effectiveTo
          currentEnd >= sorted[i + 1].effectiveFrom
        }
      })

      startsEarly and noGaps
    }
  }

  /// Verify that policy version strings are monotonically increasing
  pure def policyVersionsMonotonic(
    policies: List[CompliancePolicy]
  ): bool = {
    val sorted = policies.sortBy(p => p.effectiveFrom)

    sorted.indices().forall(i => {
      i == 0 or sorted[i - 1].version < sorted[i].version
    })
  }

  // ============================================================================
  // Utility Functions
  // ============================================================================

  /// Get the applicable policy for a given timestamp
  pure def getApplicablePolicy(
    policies: List[CompliancePolicy],
    timestamp: int
  ): List[CompliancePolicy] = {
    policies.filter(p => {
      p.effectiveFrom <= timestamp and
      (p.effectiveTo == -1 or p.effectiveTo >= timestamp)
    })
  }

  /// Check if a rule is active at a given time
  pure def ruleIsActive(rule: ComplianceRule, timestamp: int): bool = {
    rule.effectiveFrom <= timestamp and
    (rule.effectiveTo == -1 or rule.effectiveTo >= timestamp)
  }

  /// Get all rules applicable at a given time
  pure def getActiveRules(
    rules: List[ComplianceRule],
    timestamp: int
  ): List[ComplianceRule] = {
    rules.filter(r => ruleIsActive(r, timestamp))
  }

  /// Calculate decision age in time units
  pure def decisionAge(decision: ComplianceDecision, currentTime: int): int = {
    currentTime - decision.decisionTime
  }

  /// Check if a decision is within retention period
  pure def withinRetentionPeriod(
    decision: ComplianceDecision,
    currentTime: int,
    retentionPeriod: int
  ): bool = {
    decisionAge(decision, currentTime) <= retentionPeriod
  }

}
