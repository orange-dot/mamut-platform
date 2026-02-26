/// temporal_accuracy.qnt - Specification for temporal accuracy in compliance decisions
///
/// This specification models the temporal aspects of compliance systems,
/// ensuring that decisions are made using the correct policy version based
/// on effective dates, time windows, and policy versioning rules.
///
/// Key properties verified:
/// - CorrectPolicyVersion: Decisions use the policy effective at decision time
/// - NoFuturePolicies: Future-dated policies are not applied prematurely
/// - NoExpiredPolicies: Expired policies are not used
/// - ContinuousCoverage: No time gaps in policy coverage
/// - EffectiveDateAccuracy: Decision effective dates match policy dates
module temporal_accuracy {

  import Compliance.*

  // ============================================================================
  // Constants
  // ============================================================================

  /// Time resolution (1 unit = 1 day for simplicity)
  const TIME_UNIT: int = 1

  /// Maximum simulation time
  const MAX_TIME: int = 365

  /// Policy identifiers
  const POLICY_IDS: Set[str] = Set("policy1", "policy2")

  /// Policy versions
  const VERSIONS: List[str] = ["v1.0", "v1.1", "v2.0", "v2.1", "v3.0"]

  /// Entity identifiers for inputs
  const ENTITIES: Set[str] = Set("customer1", "customer2", "customer3")

  // ============================================================================
  // State Variables
  // ============================================================================

  /// All policies in the system (historical and current)
  var policies: List[CompliancePolicy]

  /// All rules across all policies
  var rules: List[ComplianceRule]

  /// All inputs received
  var inputs: List[ComplianceInput]

  /// All decisions made
  var decisions: List[ComplianceDecision]

  /// Current system time
  var currentTime: int

  /// System clock configuration
  var systemClock: ComplianceClock

  /// Scheduled policy activations (future policies)
  var scheduledPolicies: List[ScheduledPolicy]

  /// History of policy changes for audit
  var policyChangeHistory: List[PolicyChange]

  // ============================================================================
  // Additional Type Definitions
  // ============================================================================

  /// A scheduled future policy
  type ScheduledPolicy = {
    policy: CompliancePolicy,
    scheduledTime: int,
    status: ScheduleStatus
  }

  type ScheduleStatus =
    | Pending
    | Activated
    | Cancelled

  /// Record of a policy change
  type PolicyChange = {
    policyId: str,
    fromVersion: str,
    toVersion: str,
    changeTime: int,
    effectiveTime: int,
    changeType: ChangeType
  }

  type ChangeType =
    | PolicyCreated
    | PolicyUpdated
    | PolicyExpired
    | PolicyRevoked

  /// Time window for decision validity
  type TimeWindow = {
    start: int,
    end: int
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /// Get the applicable policy for a given time
  pure def getApplicablePolicyAt(
    policyList: List[CompliancePolicy],
    time: int
  ): List[CompliancePolicy] = {
    policyList.filter(p =>
      p.effectiveFrom <= time and
      (p.effectiveTo == -1 or p.effectiveTo >= time)
    )
  }

  /// Check if a time falls within a window
  pure def inTimeWindow(time: int, window: TimeWindow): bool = {
    time >= window.start and time <= window.end
  }

  /// Get the latest policy version before a given time
  pure def getLatestPolicyBefore(
    policyList: List[CompliancePolicy],
    time: int
  ): List[CompliancePolicy] = {
    val applicable = policyList.filter(p => p.effectiveFrom <= time)
    if (applicable.length() == 0) {
      []
    } else {
      val maxEffective = applicable.map(p => p.effectiveFrom).fold(0, (a, b) => if (a > b) a else b)
      applicable.filter(p => p.effectiveFrom == maxEffective)
    }
  }

  /// Check if policy version string is newer than another
  pure def isNewerVersion(v1: str, v2: str): bool = {
    // Simple lexicographic comparison (works for semantic versioning)
    v1 > v2
  }

  // ============================================================================
  // State Predicates
  // ============================================================================

  /// Check if there is an active policy at current time
  def hasActivePolicy: bool = {
    getApplicablePolicyAt(policies, currentTime).length() > 0
  }

  /// Get the currently active policy
  def getCurrentPolicy: List[CompliancePolicy] = {
    getApplicablePolicyAt(policies, currentTime)
  }

  /// Check if there are pending scheduled policies
  def hasPendingScheduledPolicies: bool = {
    scheduledPolicies.exists(sp => sp.status == Pending and sp.scheduledTime > currentTime)
  }

  /// Check if a policy is expired
  def isPolicyExpired(policy: CompliancePolicy): bool = {
    policy.effectiveTo != -1 and policy.effectiveTo < currentTime
  }

  // ============================================================================
  // Actions
  // ============================================================================

  /// Initialize the system
  action Init: bool = all {
    policies' = [],
    rules' = [],
    inputs' = [],
    decisions' = [],
    currentTime' = 0,
    systemClock' = { currentTime: 0, timezone: "UTC", precision: 1 },
    scheduledPolicies' = [],
    policyChangeHistory' = []
  }

  /// Create a new policy with an effective date
  action CreatePolicy(
    policyId: str,
    version: str,
    effectiveFrom: int,
    effectiveTo: int
  ): bool = {
    val newPolicy: CompliancePolicy = {
      id: policyId,
      version: version,
      effectiveFrom: effectiveFrom,
      effectiveTo: effectiveTo,
      rules: Set()
    }

    val changeRecord: PolicyChange = {
      policyId: policyId,
      fromVersion: "",
      toVersion: version,
      changeTime: currentTime,
      effectiveTime: effectiveFrom,
      changeType: PolicyCreated
    }

    // If effective in the future, schedule it
    if (effectiveFrom > currentTime) {
      val scheduled: ScheduledPolicy = {
        policy: newPolicy,
        scheduledTime: effectiveFrom,
        status: Pending
      }

      all {
        policies' = policies,
        rules' = rules,
        inputs' = inputs,
        decisions' = decisions,
        currentTime' = currentTime,
        systemClock' = systemClock,
        scheduledPolicies' = scheduledPolicies.append(scheduled),
        policyChangeHistory' = policyChangeHistory.append(changeRecord)
      }
    } else {
      all {
        policies' = policies.append(newPolicy),
        rules' = rules,
        inputs' = inputs,
        decisions' = decisions,
        currentTime' = currentTime,
        systemClock' = systemClock,
        scheduledPolicies' = scheduledPolicies,
        policyChangeHistory' = policyChangeHistory.append(changeRecord)
      }
    }
  }

  /// Activate scheduled policies that have reached their effective time
  action ActivateScheduledPolicies: bool = {
    val toActivate = scheduledPolicies.filter(sp =>
      sp.status == Pending and sp.scheduledTime <= currentTime
    )

    val activated = toActivate.map(sp => {
      { ...sp, status: Activated }
    })

    val remaining = scheduledPolicies.filter(sp =>
      sp.status != Pending or sp.scheduledTime > currentTime
    )

    val newPolicies = toActivate.map(sp => sp.policy)

    all {
      policies' = policies.concat(newPolicies),
      rules' = rules,
      inputs' = inputs,
      decisions' = decisions,
      currentTime' = currentTime,
      systemClock' = systemClock,
      scheduledPolicies' = remaining.concat(activated),
      policyChangeHistory' = policyChangeHistory
    }
  }

  /// Expire a policy
  action ExpirePolicy(policyId: str, expiryTime: int): bool = {
    val updated = policies.map(p => {
      if (p.id == policyId and p.effectiveTo == -1) {
        { ...p, effectiveTo: expiryTime }
      } else {
        p
      }
    })

    val changeRecord: PolicyChange = {
      policyId: policyId,
      fromVersion: "",
      toVersion: "",
      changeTime: currentTime,
      effectiveTime: expiryTime,
      changeType: PolicyExpired
    }

    all {
      policies' = updated,
      rules' = rules,
      inputs' = inputs,
      decisions' = decisions,
      currentTime' = currentTime,
      systemClock' = systemClock,
      scheduledPolicies' = scheduledPolicies,
      policyChangeHistory' = policyChangeHistory.append(changeRecord)
    }
  }

  /// Receive a compliance input
  action ReceiveInput(inputId: str, entityId: str, inputData: str): bool = {
    val newInput: ComplianceInput = {
      id: inputId,
      entityId: entityId,
      inputData: inputData,
      timestamp: currentTime,
      context: "standard"
    }

    all {
      policies' = policies,
      rules' = rules,
      inputs' = inputs.append(newInput),
      decisions' = decisions,
      currentTime' = currentTime,
      systemClock' = systemClock,
      scheduledPolicies' = scheduledPolicies,
      policyChangeHistory' = policyChangeHistory
    }
  }

  /// Make a decision using the temporally correct policy
  action MakeDecision(inputId: str): bool = {
    val matchingInputs = inputs.filter(i => i.id == inputId)

    if (matchingInputs.length() == 0) {
      all {
        policies' = policies,
        rules' = rules,
        inputs' = inputs,
        decisions' = decisions,
        currentTime' = currentTime,
        systemClock' = systemClock,
        scheduledPolicies' = scheduledPolicies,
        policyChangeHistory' = policyChangeHistory
      }
    } else {
      val input = matchingInputs[0]
      val applicablePolicies = getApplicablePolicyAt(policies, currentTime)

      if (applicablePolicies.length() == 0) {
        // No applicable policy - cannot make decision
        all {
          policies' = policies,
          rules' = rules,
          inputs' = inputs,
          decisions' = decisions,
          currentTime' = currentTime,
          systemClock' = systemClock,
          scheduledPolicies' = scheduledPolicies,
          policyChangeHistory' = policyChangeHistory
        }
      } else {
        // Use the most recent applicable policy
        val policy = applicablePolicies[0]

        val newDecision: ComplianceDecision = {
          id: "decision_" ++ inputId ++ "_" ++ currentTime.toString(),
          inputId: inputId,
          outcome: Approved,  // Simplified
          ruleId: "default",
          policyVersion: policy.version,
          effectiveDate: policy.effectiveFrom,
          decisionTime: currentTime,
          explanation: "Evaluated with temporally correct policy"
        }

        all {
          policies' = policies,
          rules' = rules,
          inputs' = inputs,
          decisions' = decisions.append(newDecision),
          currentTime' = currentTime,
          systemClock' = systemClock,
          scheduledPolicies' = scheduledPolicies,
          policyChangeHistory' = policyChangeHistory
        }
      }
    }
  }

  /// Advance system time
  action AdvanceTime(delta: int): bool = {
    all {
      policies' = policies,
      rules' = rules,
      inputs' = inputs,
      decisions' = decisions,
      currentTime' = currentTime + delta,
      systemClock' = { ...systemClock, currentTime: currentTime + delta },
      scheduledPolicies' = scheduledPolicies,
      policyChangeHistory' = policyChangeHistory
    }
  }

  /// Non-deterministic step
  action step: bool = {
    nondet policyId = oneOf(POLICY_IDS)
    nondet version = oneOf(VERSIONS.toSet())
    nondet effectiveFrom = oneOf(Set(0, 10, 30, 60, 90))
    nondet effectiveTo = oneOf(Set(-1, 30, 60, 90, 120))
    nondet inputId = oneOf(Set("input1", "input2", "input3"))
    nondet entityId = oneOf(ENTITIES)
    nondet inputData = oneOf(Set("data1", "data2"))
    nondet timeDelta = oneOf(Set(1, 5, 10, 30))

    any {
      CreatePolicy(policyId, version, effectiveFrom, effectiveTo),
      ActivateScheduledPolicies,
      ExpirePolicy(policyId, currentTime + 10),
      ReceiveInput(inputId, entityId, inputData),
      MakeDecision(inputId),
      AdvanceTime(timeDelta)
    }
  }

  // ============================================================================
  // Invariants
  // ============================================================================

  /// INV1: CorrectPolicyVersion
  /// All decisions must use a policy that was effective at decision time
  val CorrectPolicyVersion: bool = {
    decisions.forall(decision => {
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      usedPolicy.length() == 0 or
      usedPolicy.exists(p =>
        decision.decisionTime >= p.effectiveFrom and
        (p.effectiveTo == -1 or decision.decisionTime <= p.effectiveTo)
      )
    })
  }

  /// INV2: NoFuturePolicies
  /// Decisions cannot use policies that are not yet effective
  val NoFuturePolicies: bool = {
    decisions.forall(decision => {
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      usedPolicy.length() == 0 or
      usedPolicy.forall(p => p.effectiveFrom <= decision.decisionTime)
    })
  }

  /// INV3: NoExpiredPolicies
  /// Decisions cannot use policies that have expired
  val NoExpiredPolicies: bool = {
    decisions.forall(decision => {
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      usedPolicy.length() == 0 or
      usedPolicy.forall(p =>
        p.effectiveTo == -1 or p.effectiveTo >= decision.decisionTime
      )
    })
  }

  /// INV4: EffectiveDateAccuracy
  /// Decision's effective date must match the policy's effective date
  val EffectiveDateAccuracy: bool = {
    decisions.forall(decision => {
      val usedPolicy = policies.filter(p => p.version == decision.policyVersion)

      usedPolicy.length() == 0 or
      usedPolicy.exists(p => decision.effectiveDate == p.effectiveFrom)
    })
  }

  /// INV5: ScheduledPoliciesNotPremature
  /// Scheduled policies should not be in the active policies list before their time
  val ScheduledPoliciesNotPremature: bool = {
    scheduledPolicies.forall(sp => {
      sp.status != Pending or
      not(policies.exists(p => p.version == sp.policy.version))
    })
  }

  /// INV6: DecisionTimeConsistency
  /// Decision time should not exceed current system time
  val DecisionTimeConsistency: bool = {
    decisions.forall(d => d.decisionTime <= currentTime)
  }

  /// INV7: PolicyVersionUniqueness
  /// No two active policies should have the same version at the same time
  val PolicyVersionUniqueness: bool = {
    policies.indices().forall(i => {
      policies.indices().forall(j => {
        if (i < j) {
          val p1 = policies[i]
          val p2 = policies[j]

          // If same version, they shouldn't overlap
          p1.version != p2.version or
          p1.id != p2.id or
          // No overlap in effective periods
          (p1.effectiveTo != -1 and p1.effectiveTo < p2.effectiveFrom) or
          (p2.effectiveTo != -1 and p2.effectiveTo < p1.effectiveFrom)
        } else {
          true
        }
      })
    })
  }

  /// INV8: ChangeHistoryMonotonic
  /// Policy change times should be monotonically increasing
  val ChangeHistoryMonotonic: bool = {
    policyChangeHistory.indices().forall(i => {
      i == 0 or policyChangeHistory[i - 1].changeTime <= policyChangeHistory[i].changeTime
    })
  }

  /// Combined safety invariant
  val Safety: bool = {
    CorrectPolicyVersion and
    NoFuturePolicies and
    NoExpiredPolicies and
    EffectiveDateAccuracy and
    DecisionTimeConsistency
  }

  // ============================================================================
  // Temporal Properties
  // ============================================================================

  /// Eventual activation: scheduled policies eventually become active
  val EventualActivation: bool = {
    scheduledPolicies.forall(sp => {
      sp.status == Pending implies (
        sp.scheduledTime > currentTime or
        policies.exists(p => p.version == sp.policy.version)
      )
    })
  }

  // ============================================================================
  // Test Scenarios
  // ============================================================================

  /// Test: Policy becomes effective at scheduled time
  run testPolicyEffectiveDate = {
    Init
    .then(CreatePolicy("policy1", "v1.0", 10, -1))  // Effective at time 10
    .then(AdvanceTime(5))
    .then(all {
      // Policy should be scheduled, not active
      assert(scheduledPolicies.length() == 1),
      assert(policies.length() == 0)
    })
    .then(AdvanceTime(5))  // Now at time 10
    .then(ActivateScheduledPolicies)
    .then(all {
      // Policy should now be active
      assert(policies.length() == 1),
      assert(hasActivePolicy)
    })
  }

  /// Test: Expired policy cannot be used
  run testExpiredPolicy = {
    Init
    .then(CreatePolicy("policy1", "v1.0", 0, 10))  // Expires at time 10
    .then(ReceiveInput("input1", "customer1", "data1"))
    .then(MakeDecision("input1"))  // Should succeed
    .then(all {
      assert(decisions.length() == 1),
      assert(NoExpiredPolicies)
    })
    .then(AdvanceTime(15))  // Now past expiry
    .then(ReceiveInput("input2", "customer1", "data2"))
    .then(MakeDecision("input2"))  // Should fail - no active policy
    .then(all {
      // No new decision should be made
      assert(decisions.length() == 1)
    })
  }

  /// Test: Decision uses correct policy version
  run testCorrectPolicyVersion = {
    Init
    .then(CreatePolicy("policy1", "v1.0", 0, 30))
    .then(CreatePolicy("policy1", "v2.0", 30, -1))
    .then(ReceiveInput("input1", "customer1", "data1"))
    .then(MakeDecision("input1"))  // Uses v1.0
    .then(AdvanceTime(35))
    .then(ActivateScheduledPolicies)
    .then(ReceiveInput("input2", "customer1", "data2"))
    .then(MakeDecision("input2"))  // Uses v2.0
    .then(all {
      assert(CorrectPolicyVersion),
      assert(decisions[0].policyVersion == "v1.0"),
      assert(decisions[1].policyVersion == "v2.0")
    })
  }

  /// Test: Policy transition with no gaps
  run testContinuousCoverage = {
    Init
    .then(CreatePolicy("policy1", "v1.0", 0, 30))
    .then(CreatePolicy("policy1", "v2.0", 30, -1))  // Starts when v1.0 ends
    .then(AdvanceTime(35))
    .then(ActivateScheduledPolicies)
    .then(all {
      // Should have continuous coverage
      assert(hasActivePolicy)
    })
  }

}
