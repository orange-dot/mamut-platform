//! # mamut-report
//!
//! Report generation module for audit-grade artifacts.
//!
//! This crate provides comprehensive report generation capabilities for the
//! Mamut verification framework, producing audit-ready documentation of test
//! results, violations, and reproducibility information.
//!
//! ## Features
//!
//! - **Multiple Output Formats**: Generate reports in JSON, HTML, and Markdown
//! - **Timeline Visualization**: Visual representation of test execution
//! - **Reproducibility Bundles**: Docker-based reproduction packages
//! - **Audit-Grade Documentation**: Detailed violation reports with evidence
//!
//! ## Report Types
//!
//! - [`AuditReport`]: Complete audit report with all findings
//! - [`ViolationReport`]: Detailed violation documentation
//! - [`ReproBundle`]: Reproducibility package for violations
//!
//! ## Example
//!
//! ```rust,ignore
//! use mamut_report::{
//!     AuditReport, ReportSummary, TestConfiguration,
//!     JsonReportGenerator, HtmlReportGenerator, MarkdownReportGenerator,
//! };
//! use std::time::Duration;
//!
//! // Create a report
//! let summary = ReportSummary::new("test-run-1".into(), "Linearizability Test".into());
//! let config = TestConfiguration {
//!     name: "linearizability-test".into(),
//!     consistency_model: "linearizability".into(),
//!     system_under_test: "my-database".into(),
//!     concurrency: 4,
//!     timeout: Duration::from_secs(60),
//!     ..Default::default()
//! };
//!
//! let report = AuditReport::builder("test-run-1".into(), "Linearizability Test".into())
//!     .with_test_configuration(config)
//!     .with_duration(Duration::from_secs(30))
//!     .with_total_operations(1000)
//!     .build()
//!     .unwrap();
//!
//! // Generate reports in different formats
//! let json_gen = JsonReportGenerator::new();
//! let json = json_gen.generate(&report).unwrap();
//!
//! let html_gen = HtmlReportGenerator::new();
//! let html = html_gen.generate(&report).unwrap();
//!
//! let md_gen = MarkdownReportGenerator::new();
//! let markdown = md_gen.generate(&report).unwrap();
//! ```
//!
//! ## Reproducibility Bundles
//!
//! Generate Docker-based reproducibility packages:
//!
//! ```rust,ignore
//! use mamut_report::{ReproBundleGenerator, ReproBundleConfig};
//!
//! let config = ReproBundleConfig {
//!     test_runner_image: "mamut/test-runner:latest".into(),
//!     ..Default::default()
//! };
//!
//! let generator = ReproBundleGenerator::with_config(config);
//! let bundle = generator.generate(&report).unwrap();
//!
//! // Write to directory
//! generator.write_to_directory(&bundle, "./repro-bundle").unwrap();
//!
//! // Or create a tar.gz archive
//! let file = std::fs::File::create("repro-bundle.tar.gz").unwrap();
//! generator.write_to_archive(&bundle, file).unwrap();
//! ```
//!
//! ## Timeline Visualization
//!
//! Generate timeline data for visualization:
//!
//! ```rust,ignore
//! use mamut_report::timeline::{TimelineGenerator, TimelineData, timeline_to_svg};
//!
//! let timeline_gen = TimelineGenerator::new()
//!     .with_operations(true)
//!     .with_faults(true)
//!     .with_violations(true);
//!
//! let timeline = timeline_gen.generate_from_report(&report);
//! let svg = timeline_to_svg(&timeline, 800, 200);
//! ```

pub mod html;
pub mod json;
pub mod markdown;
pub mod repro;
pub mod timeline;
pub mod types;

// Re-export main types at crate root for convenience
pub use html::{HtmlReportConfig, HtmlReportGenerator};
pub use json::{JsonFormat, JsonLinesGenerator, JsonReportConfig, JsonReportGenerator};
pub use markdown::{MarkdownReportConfig, MarkdownReportGenerator};
pub use repro::{ReproBundleConfig, ReproBundleGenerator, VolumeMount};
pub use timeline::{
    LegendEntry, TimelineData, TimelineEvent, TimelineEventType, TimelineGenerator, TimelineLane,
    TimelineMarker, TimelineSpan,
};
pub use types::{
    AuditReport, AuditReportBuilder, CoverageReport, Evidence, EvidenceType, FaultConfiguration,
    FaultCoverage, HistoryAnalysis, MinimalHistoryRef, OperationCoverage, Recommendation,
    ReportError, ReportId, ReportSummary, ReproBundle, RootCauseAnalysis, Severity,
    TestConfiguration, ViolationCategory, ViolationId, ViolationReport,
};

/// Convenience function to generate a report in all formats.
pub fn generate_all_formats(
    report: &AuditReport,
) -> Result<(String, String, String), ReportError> {
    let json = JsonReportGenerator::new().generate(report)?;
    let html = HtmlReportGenerator::new().generate(report)?;
    let markdown = MarkdownReportGenerator::new().generate(report)?;

    Ok((json, html, markdown))
}

/// Convenience function to write reports to a directory.
pub fn write_reports_to_directory(
    report: &AuditReport,
    dir: impl AsRef<std::path::Path>,
) -> Result<(), ReportError> {
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir)?;

    let base_name = format!("report-{}", report.summary.report_id);

    JsonReportGenerator::new().generate_to_file(report, dir.join(format!("{}.json", base_name)))?;

    HtmlReportGenerator::new().generate_to_file(report, dir.join(format!("{}.html", base_name)))?;

    MarkdownReportGenerator::new()
        .generate_to_file(report, dir.join(format!("{}.md", base_name)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_report() -> AuditReport {
        let summary = ReportSummary::new("test-run-1".to_string(), "Test Report".to_string());
        let config = TestConfiguration {
            name: "test".to_string(),
            description: Some("Test description".to_string()),
            consistency_model: "linearizability".to_string(),
            system_under_test: "test-db".to_string(),
            system_version: Some("1.0.0".to_string()),
            concurrency: 4,
            timeout: Duration::from_secs(60),
            seed: Some(42),
            fault_config: None,
            parameters: HashMap::new(),
        };

        AuditReport::new(summary, config)
    }

    #[test]
    fn test_generate_all_formats() {
        let report = create_test_report();
        let (json, html, markdown) = generate_all_formats(&report).unwrap();

        assert!(json.contains("test-run-1"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(markdown.contains("# Test Report"));
    }

    #[test]
    fn test_report_builder() {
        let config = TestConfiguration {
            name: "builder-test".to_string(),
            description: None,
            consistency_model: "linearizability".to_string(),
            system_under_test: "test-db".to_string(),
            system_version: None,
            concurrency: 4,
            timeout: Duration::from_secs(60),
            seed: Some(42),
            fault_config: None,
            parameters: HashMap::new(),
        };

        let report = AuditReport::builder("test-run-1".to_string(), "Builder Test".to_string())
            .with_test_configuration(config)
            .with_duration(Duration::from_secs(30))
            .with_total_operations(500)
            .with_total_faults(10)
            .build()
            .unwrap();

        assert!(report.passed());
        assert_eq!(report.summary.total_operations, 500);
        assert_eq!(report.summary.total_faults, 10);
        assert_eq!(report.summary.test_duration, Duration::from_secs(30));
    }

    #[test]
    fn test_violation_report_creation() {
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

        let rca = RootCauseAnalysis {
            cause_category: "Concurrency".to_string(),
            analysis: "Race condition detected".to_string(),
            contributing_factors: vec!["High load".to_string()],
            suggested_fixes: vec!["Add locking".to_string()],
            code_locations: vec!["src/lib.rs:42".to_string()],
            confidence: 0.85,
        };

        let violation = violation.with_root_cause_analysis(rca);

        assert!(violation.root_cause_analysis.is_some());
        assert_eq!(
            violation.root_cause_analysis.as_ref().unwrap().confidence,
            0.85
        );
    }

    #[test]
    fn test_report_with_violations() {
        let config = TestConfiguration {
            name: "violation-test".to_string(),
            description: None,
            consistency_model: "linearizability".to_string(),
            system_under_test: "test-db".to_string(),
            system_version: None,
            concurrency: 4,
            timeout: Duration::from_secs(60),
            seed: Some(42),
            fault_config: None,
            parameters: HashMap::new(),
        };

        let violation = ViolationReport::new(
            Severity::Critical,
            ViolationCategory::DataLoss,
            "Data was lost during failover".to_string(),
            MinimalHistoryRef {
                id: "h1".to_string(),
                operation_ids: vec!["op1".to_string()],
                original_size: 50,
                minimized_size: 1,
                description: "Minimal history".to_string(),
            },
        );

        let report = AuditReport::builder("test-run-1".to_string(), "Violation Test".to_string())
            .with_test_configuration(config)
            .with_violation(violation)
            .build()
            .unwrap();

        assert!(!report.passed());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].severity, Severity::Critical);
    }

    #[test]
    fn test_timeline_generation() {
        let report = create_test_report();
        let generator = TimelineGenerator::new();
        let timeline = generator.generate_from_report(&report);

        assert!(!timeline.title.is_empty());
    }

    #[test]
    fn test_repro_bundle_generation() {
        let report = create_test_report();
        let generator = ReproBundleGenerator::new();
        let bundle = generator.generate(&report).unwrap();

        assert!(!bundle.docker_compose.is_empty());
        assert!(!bundle.test_script.is_empty());
        assert!(!bundle.instructions.is_empty());
    }
}
