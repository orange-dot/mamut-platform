//! Report types for audit-grade artifacts.
//!
//! This module defines the core types for generating comprehensive audit reports
//! that document test runs, violations, coverage, and reproducibility bundles.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a violation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViolationId(pub Uuid);

impl ViolationId {
    /// Create a new random violation ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a violation ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for ViolationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ViolationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a report.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReportId(pub Uuid);

impl ReportId {
    /// Create a new random report ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ReportId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ReportId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Severity level of a violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Critical - data loss, corruption, or security vulnerability
    Critical,
    /// High - significant correctness issue
    High,
    /// Medium - correctness issue under specific conditions
    Medium,
    /// Low - minor issue, potential edge case
    Low,
    /// Info - informational finding
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

impl Severity {
    /// Get the numeric weight for sorting (higher = more severe).
    pub fn weight(&self) -> u8 {
        match self {
            Severity::Critical => 5,
            Severity::High => 4,
            Severity::Medium => 3,
            Severity::Low => 2,
            Severity::Info => 1,
        }
    }

    /// Get the CSS class name for HTML rendering.
    pub fn css_class(&self) -> &'static str {
        match self {
            Severity::Critical => "severity-critical",
            Severity::High => "severity-high",
            Severity::Medium => "severity-medium",
            Severity::Low => "severity-low",
            Severity::Info => "severity-info",
        }
    }
}

/// Category of violation detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationCategory {
    /// Linearizability violation - no valid linearization exists
    Linearizability,
    /// Sequential consistency violation
    SequentialConsistency,
    /// Causal consistency violation
    CausalConsistency,
    /// Serializability violation
    Serializability,
    /// Real-time order violation
    RealTimeOrder,
    /// Program order violation
    ProgramOrder,
    /// Stale read - read returned an old value
    StaleRead,
    /// Future read - read returned a value before it was written
    FutureRead,
    /// Write conflict - concurrent writes not properly ordered
    WriteConflict,
    /// Invariant violation - custom invariant broken
    Invariant,
    /// Recovery failure - system didn't recover correctly
    RecoveryFailure,
    /// Data loss - data was lost
    DataLoss,
    /// Custom category
    Custom(String),
}

impl fmt::Display for ViolationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationCategory::Linearizability => write!(f, "Linearizability"),
            ViolationCategory::SequentialConsistency => write!(f, "Sequential Consistency"),
            ViolationCategory::CausalConsistency => write!(f, "Causal Consistency"),
            ViolationCategory::Serializability => write!(f, "Serializability"),
            ViolationCategory::RealTimeOrder => write!(f, "Real-Time Order"),
            ViolationCategory::ProgramOrder => write!(f, "Program Order"),
            ViolationCategory::StaleRead => write!(f, "Stale Read"),
            ViolationCategory::FutureRead => write!(f, "Future Read"),
            ViolationCategory::WriteConflict => write!(f, "Write Conflict"),
            ViolationCategory::Invariant => write!(f, "Invariant"),
            ViolationCategory::RecoveryFailure => write!(f, "Recovery Failure"),
            ViolationCategory::DataLoss => write!(f, "Data Loss"),
            ViolationCategory::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Reference to a minimal history that demonstrates a violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalHistoryRef {
    /// Unique identifier for this minimal history.
    pub id: String,
    /// Operation IDs in the minimal history.
    pub operation_ids: Vec<String>,
    /// Original history size before minimization.
    pub original_size: usize,
    /// Minimized history size.
    pub minimized_size: usize,
    /// Description of what this history demonstrates.
    pub description: String,
}

/// Root cause analysis for a violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseAnalysis {
    /// Primary cause category.
    pub cause_category: String,
    /// Detailed analysis text.
    pub analysis: String,
    /// Contributing factors.
    pub contributing_factors: Vec<String>,
    /// Potential fix suggestions.
    pub suggested_fixes: Vec<String>,
    /// Related code locations (file:line).
    pub code_locations: Vec<String>,
    /// Confidence level (0.0 - 1.0).
    pub confidence: f64,
}

/// Evidence supporting a violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence.
    pub evidence_type: EvidenceType,
    /// Evidence description.
    pub description: String,
    /// Raw data as JSON.
    pub data: serde_json::Value,
    /// Timestamp when evidence was collected.
    pub collected_at: DateTime<Utc>,
}

/// Type of evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Operation trace.
    OperationTrace,
    /// State snapshot.
    StateSnapshot,
    /// Network capture.
    NetworkCapture,
    /// Log excerpt.
    LogExcerpt,
    /// Timing data.
    TimingData,
    /// Counterexample.
    Counterexample,
    /// Vector clock state.
    VectorClock,
    /// Custom evidence type.
    Custom(String),
}

/// A detailed violation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationReport {
    /// Unique identifier for this violation.
    pub id: ViolationId,
    /// Severity of the violation.
    pub severity: Severity,
    /// Category of violation.
    pub category: ViolationCategory,
    /// Human-readable description.
    pub description: String,
    /// Reference to minimal history demonstrating this violation.
    pub minimal_history: MinimalHistoryRef,
    /// Root cause analysis (if available).
    pub root_cause_analysis: Option<RootCauseAnalysis>,
    /// Evidence supporting this violation.
    pub evidence: Vec<Evidence>,
    /// Timestamp when violation was detected.
    pub detected_at: DateTime<Utc>,
    /// Operations involved in the violation.
    pub involved_operations: Vec<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ViolationReport {
    /// Create a new violation report.
    pub fn new(
        severity: Severity,
        category: ViolationCategory,
        description: String,
        minimal_history: MinimalHistoryRef,
    ) -> Self {
        Self {
            id: ViolationId::new(),
            severity,
            category,
            description,
            minimal_history,
            root_cause_analysis: None,
            evidence: Vec::new(),
            detected_at: Utc::now(),
            involved_operations: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add root cause analysis.
    pub fn with_root_cause_analysis(mut self, analysis: RootCauseAnalysis) -> Self {
        self.root_cause_analysis = Some(analysis);
        self
    }

    /// Add evidence.
    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Add multiple pieces of evidence.
    pub fn with_evidence_list(mut self, evidence: Vec<Evidence>) -> Self {
        self.evidence.extend(evidence);
        self
    }

    /// Add involved operations.
    pub fn with_involved_operations(mut self, ops: Vec<String>) -> Self {
        self.involved_operations = ops;
        self
    }
}

/// Summary of the audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    /// Report identifier.
    pub report_id: ReportId,
    /// Report title.
    pub title: String,
    /// Report generation timestamp.
    pub generated_at: DateTime<Utc>,
    /// Test run identifier.
    pub test_run_id: String,
    /// Overall result (pass/fail).
    pub passed: bool,
    /// Total number of violations.
    pub total_violations: usize,
    /// Violations by severity.
    pub violations_by_severity: HashMap<String, usize>,
    /// Violations by category.
    pub violations_by_category: HashMap<String, usize>,
    /// Total test duration.
    pub test_duration: Duration,
    /// Total operations executed.
    pub total_operations: u64,
    /// Total faults injected.
    pub total_faults: u64,
    /// Version of the verification framework.
    pub framework_version: String,
}

impl ReportSummary {
    /// Create a new report summary.
    pub fn new(test_run_id: String, title: String) -> Self {
        Self {
            report_id: ReportId::new(),
            title,
            generated_at: Utc::now(),
            test_run_id,
            passed: true,
            total_violations: 0,
            violations_by_severity: HashMap::new(),
            violations_by_category: HashMap::new(),
            test_duration: Duration::ZERO,
            total_operations: 0,
            total_faults: 0,
            framework_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Update summary based on violations.
    pub fn update_from_violations(&mut self, violations: &[ViolationReport]) {
        self.total_violations = violations.len();
        self.passed = violations.is_empty();

        self.violations_by_severity.clear();
        self.violations_by_category.clear();

        for violation in violations {
            *self
                .violations_by_severity
                .entry(violation.severity.to_string())
                .or_insert(0) += 1;

            *self
                .violations_by_category
                .entry(violation.category.to_string())
                .or_insert(0) += 1;
        }
    }
}

/// Test configuration details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfiguration {
    /// Test name.
    pub name: String,
    /// Test description.
    pub description: Option<String>,
    /// Consistency model being verified.
    pub consistency_model: String,
    /// Target system under test.
    pub system_under_test: String,
    /// System version.
    pub system_version: Option<String>,
    /// Number of concurrent processes/threads.
    pub concurrency: u32,
    /// Test duration limit.
    pub timeout: Duration,
    /// Random seed for reproducibility.
    pub seed: Option<u64>,
    /// Fault injection configuration.
    pub fault_config: Option<FaultConfiguration>,
    /// Additional parameters.
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Fault injection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultConfiguration {
    /// Enabled fault types.
    pub enabled_faults: Vec<String>,
    /// Fault injection probability.
    pub injection_probability: f64,
    /// Maximum concurrent faults.
    pub max_concurrent_faults: u32,
    /// Fault-specific configurations.
    pub fault_parameters: HashMap<String, serde_json::Value>,
}

/// History analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryAnalysis {
    /// Total events in history.
    pub total_events: u64,
    /// Total operations.
    pub total_operations: u64,
    /// Operations by type.
    pub operations_by_type: HashMap<String, u64>,
    /// Operations by process.
    pub operations_by_process: HashMap<String, u64>,
    /// Faults injected.
    pub faults_injected: u64,
    /// Faults by type.
    pub faults_by_type: HashMap<String, u64>,
    /// History time span.
    pub time_span: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Peak concurrency observed.
    pub peak_concurrency: u32,
    /// Average operation latency.
    pub average_latency: Option<Duration>,
    /// P99 operation latency.
    pub p99_latency: Option<Duration>,
}

impl Default for HistoryAnalysis {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_operations: 0,
            operations_by_type: HashMap::new(),
            operations_by_process: HashMap::new(),
            faults_injected: 0,
            faults_by_type: HashMap::new(),
            time_span: None,
            peak_concurrency: 0,
            average_latency: None,
            p99_latency: None,
        }
    }
}

/// Coverage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// State space coverage percentage (0.0 - 100.0).
    pub state_space_coverage: f64,
    /// Number of unique states explored.
    pub states_explored: u64,
    /// Estimated total state space size.
    pub estimated_state_space: Option<u64>,
    /// Operation type coverage.
    pub operation_coverage: HashMap<String, OperationCoverage>,
    /// Fault scenario coverage.
    pub fault_coverage: HashMap<String, FaultCoverage>,
    /// Branch coverage (if available).
    pub branch_coverage: Option<f64>,
    /// Path coverage (if available).
    pub path_coverage: Option<f64>,
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self {
            state_space_coverage: 0.0,
            states_explored: 0,
            estimated_state_space: None,
            operation_coverage: HashMap::new(),
            fault_coverage: HashMap::new(),
            branch_coverage: None,
            path_coverage: None,
        }
    }
}

/// Coverage for a specific operation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCoverage {
    /// Operation type name.
    pub operation_type: String,
    /// Number of times executed.
    pub execution_count: u64,
    /// Number of unique input combinations.
    pub unique_inputs: u64,
    /// Success rate.
    pub success_rate: f64,
    /// Error scenarios covered.
    pub error_scenarios: Vec<String>,
}

/// Coverage for a fault scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultCoverage {
    /// Fault type name.
    pub fault_type: String,
    /// Number of times injected.
    pub injection_count: u64,
    /// Number of times recovery succeeded.
    pub successful_recoveries: u64,
    /// Recovery scenarios tested.
    pub recovery_scenarios: Vec<String>,
}

/// Recommendation for improvement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation ID.
    pub id: String,
    /// Priority (1 = highest).
    pub priority: u32,
    /// Recommendation title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Category (e.g., "reliability", "performance", "testing").
    pub category: String,
    /// Related violations (if any).
    pub related_violations: Vec<ViolationId>,
    /// Suggested action items.
    pub action_items: Vec<String>,
    /// Expected impact.
    pub expected_impact: String,
}

/// Reproducibility bundle for recreating violations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproBundle {
    /// Docker Compose configuration.
    pub docker_compose: String,
    /// Seed data (binary).
    #[serde(with = "base64_serde")]
    pub seed_data: Vec<u8>,
    /// Test script to run.
    pub test_script: String,
    /// Description of expected violation.
    pub expected_violation: String,
    /// Environment variables required.
    pub environment: HashMap<String, String>,
    /// Additional files included in the bundle.
    pub additional_files: HashMap<String, String>,
    /// Instructions for reproduction.
    pub instructions: String,
    /// Bundle creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl ReproBundle {
    /// Create a new empty repro bundle.
    pub fn new() -> Self {
        Self {
            docker_compose: String::new(),
            seed_data: Vec::new(),
            test_script: String::new(),
            expected_violation: String::new(),
            environment: HashMap::new(),
            additional_files: HashMap::new(),
            instructions: String::new(),
            created_at: Utc::now(),
        }
    }
}

impl Default for ReproBundle {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report summary.
    pub summary: ReportSummary,
    /// Test configuration.
    pub test_configuration: TestConfiguration,
    /// History analysis results.
    pub history_analysis: HistoryAnalysis,
    /// Violations found.
    pub violations: Vec<ViolationReport>,
    /// Coverage information.
    pub coverage: CoverageReport,
    /// Recommendations.
    pub recommendations: Vec<Recommendation>,
    /// Reproducibility bundle.
    pub repro_bundle: ReproBundle,
}

impl AuditReport {
    /// Create a new audit report.
    pub fn new(summary: ReportSummary, test_configuration: TestConfiguration) -> Self {
        Self {
            summary,
            test_configuration,
            history_analysis: HistoryAnalysis::default(),
            violations: Vec::new(),
            coverage: CoverageReport::default(),
            recommendations: Vec::new(),
            repro_bundle: ReproBundle::new(),
        }
    }

    /// Create a builder for constructing reports.
    pub fn builder(test_run_id: String, title: String) -> AuditReportBuilder {
        AuditReportBuilder::new(test_run_id, title)
    }

    /// Check if the audit passed (no violations).
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }

    /// Get violations sorted by severity (most severe first).
    pub fn violations_by_severity(&self) -> Vec<&ViolationReport> {
        let mut violations: Vec<_> = self.violations.iter().collect();
        violations.sort_by(|a, b| b.severity.weight().cmp(&a.severity.weight()));
        violations
    }

    /// Get critical and high severity violations.
    pub fn critical_violations(&self) -> Vec<&ViolationReport> {
        self.violations
            .iter()
            .filter(|v| matches!(v.severity, Severity::Critical | Severity::High))
            .collect()
    }

    /// Update summary from current violations.
    pub fn update_summary(&mut self) {
        self.summary.update_from_violations(&self.violations);
    }
}

/// Builder for constructing audit reports.
pub struct AuditReportBuilder {
    summary: ReportSummary,
    test_configuration: Option<TestConfiguration>,
    history_analysis: HistoryAnalysis,
    violations: Vec<ViolationReport>,
    coverage: CoverageReport,
    recommendations: Vec<Recommendation>,
    repro_bundle: ReproBundle,
}

impl AuditReportBuilder {
    /// Create a new builder.
    pub fn new(test_run_id: String, title: String) -> Self {
        Self {
            summary: ReportSummary::new(test_run_id, title),
            test_configuration: None,
            history_analysis: HistoryAnalysis::default(),
            violations: Vec::new(),
            coverage: CoverageReport::default(),
            recommendations: Vec::new(),
            repro_bundle: ReproBundle::new(),
        }
    }

    /// Set test configuration.
    pub fn with_test_configuration(mut self, config: TestConfiguration) -> Self {
        self.test_configuration = Some(config);
        self
    }

    /// Set history analysis.
    pub fn with_history_analysis(mut self, analysis: HistoryAnalysis) -> Self {
        self.history_analysis = analysis;
        self
    }

    /// Add a violation.
    pub fn with_violation(mut self, violation: ViolationReport) -> Self {
        self.violations.push(violation);
        self
    }

    /// Add multiple violations.
    pub fn with_violations(mut self, violations: Vec<ViolationReport>) -> Self {
        self.violations.extend(violations);
        self
    }

    /// Set coverage report.
    pub fn with_coverage(mut self, coverage: CoverageReport) -> Self {
        self.coverage = coverage;
        self
    }

    /// Add a recommendation.
    pub fn with_recommendation(mut self, recommendation: Recommendation) -> Self {
        self.recommendations.push(recommendation);
        self
    }

    /// Add multiple recommendations.
    pub fn with_recommendations(mut self, recommendations: Vec<Recommendation>) -> Self {
        self.recommendations.extend(recommendations);
        self
    }

    /// Set repro bundle.
    pub fn with_repro_bundle(mut self, bundle: ReproBundle) -> Self {
        self.repro_bundle = bundle;
        self
    }

    /// Set test duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.summary.test_duration = duration;
        self
    }

    /// Set total operations.
    pub fn with_total_operations(mut self, count: u64) -> Self {
        self.summary.total_operations = count;
        self
    }

    /// Set total faults.
    pub fn with_total_faults(mut self, count: u64) -> Self {
        self.summary.total_faults = count;
        self
    }

    /// Build the audit report.
    pub fn build(mut self) -> Result<AuditReport, ReportError> {
        let test_configuration = self
            .test_configuration
            .ok_or(ReportError::MissingConfiguration)?;

        self.summary.update_from_violations(&self.violations);

        Ok(AuditReport {
            summary: self.summary,
            test_configuration,
            history_analysis: self.history_analysis,
            violations: self.violations,
            coverage: self.coverage,
            recommendations: self.recommendations,
            repro_bundle: self.repro_bundle,
        })
    }
}

/// Error types for report generation.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// Missing required configuration.
    #[error("Missing test configuration")]
    MissingConfiguration,

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Template error.
    #[error("Template error: {0}")]
    TemplateError(String),

    /// Invalid report data.
    #[error("Invalid report data: {0}")]
    InvalidData(String),
}

/// Custom serialization for binary data using base64.
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        let encoded = STANDARD.encode(data);
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD
            .decode(&encoded)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical.weight() > Severity::High.weight());
        assert!(Severity::High.weight() > Severity::Medium.weight());
        assert!(Severity::Medium.weight() > Severity::Low.weight());
        assert!(Severity::Low.weight() > Severity::Info.weight());
    }

    #[test]
    fn test_violation_report_builder() {
        let minimal_history = MinimalHistoryRef {
            id: "test-history".to_string(),
            operation_ids: vec!["op1".to_string(), "op2".to_string()],
            original_size: 100,
            minimized_size: 2,
            description: "Test history".to_string(),
        };

        let violation = ViolationReport::new(
            Severity::High,
            ViolationCategory::Linearizability,
            "Test violation".to_string(),
            minimal_history,
        );

        assert_eq!(violation.severity, Severity::High);
        assert_eq!(violation.category, ViolationCategory::Linearizability);
    }

    #[test]
    fn test_report_summary_update() {
        let mut summary = ReportSummary::new("test-run-1".to_string(), "Test Report".to_string());

        let violations = vec![
            ViolationReport::new(
                Severity::Critical,
                ViolationCategory::DataLoss,
                "Data loss".to_string(),
                MinimalHistoryRef {
                    id: "h1".to_string(),
                    operation_ids: vec![],
                    original_size: 10,
                    minimized_size: 2,
                    description: "".to_string(),
                },
            ),
            ViolationReport::new(
                Severity::High,
                ViolationCategory::Linearizability,
                "Linearizability violation".to_string(),
                MinimalHistoryRef {
                    id: "h2".to_string(),
                    operation_ids: vec![],
                    original_size: 20,
                    minimized_size: 3,
                    description: "".to_string(),
                },
            ),
        ];

        summary.update_from_violations(&violations);

        assert_eq!(summary.total_violations, 2);
        assert!(!summary.passed);
        assert_eq!(summary.violations_by_severity.get("CRITICAL"), Some(&1));
        assert_eq!(summary.violations_by_severity.get("HIGH"), Some(&1));
    }

    #[test]
    fn test_audit_report_builder() {
        let config = TestConfiguration {
            name: "linearizability-test".to_string(),
            description: Some("Test linearizability".to_string()),
            consistency_model: "linearizability".to_string(),
            system_under_test: "test-db".to_string(),
            system_version: Some("1.0.0".to_string()),
            concurrency: 4,
            timeout: Duration::from_secs(60),
            seed: Some(42),
            fault_config: None,
            parameters: HashMap::new(),
        };

        let report = AuditReport::builder("test-run-1".to_string(), "Test Report".to_string())
            .with_test_configuration(config)
            .with_duration(Duration::from_secs(30))
            .with_total_operations(1000)
            .build()
            .unwrap();

        assert!(report.passed());
        assert_eq!(report.summary.total_operations, 1000);
        assert_eq!(report.summary.test_duration, Duration::from_secs(30));
    }
}
