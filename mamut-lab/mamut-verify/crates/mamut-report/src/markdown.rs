//! Markdown report generation.
//!
//! This module provides Markdown output for audit reports, suitable for
//! inclusion in documentation, GitHub issues, or other Markdown-aware systems.

use std::io::Write;
use std::path::Path;

use tracing::instrument;

use crate::types::{AuditReport, ReportError, ViolationReport};

/// Configuration for Markdown report generation.
#[derive(Debug, Clone)]
pub struct MarkdownReportConfig {
    /// Include table of contents.
    pub include_toc: bool,
    /// Include summary section.
    pub include_summary: bool,
    /// Include configuration section.
    pub include_configuration: bool,
    /// Include violations section.
    pub include_violations: bool,
    /// Include coverage section.
    pub include_coverage: bool,
    /// Include recommendations section.
    pub include_recommendations: bool,
    /// Include repro bundle section.
    pub include_repro_bundle: bool,
    /// Maximum violations to show in detail (0 = unlimited).
    pub max_violations_detail: usize,
    /// Use GitHub-flavored Markdown extensions.
    pub github_flavored: bool,
    /// Include badges (for GitHub).
    pub include_badges: bool,
    /// Heading level offset (0 = start with #, 1 = start with ##, etc.).
    pub heading_offset: usize,
}

impl Default for MarkdownReportConfig {
    fn default() -> Self {
        Self {
            include_toc: true,
            include_summary: true,
            include_configuration: true,
            include_violations: true,
            include_coverage: true,
            include_recommendations: true,
            include_repro_bundle: true,
            max_violations_detail: 10,
            github_flavored: true,
            include_badges: true,
            heading_offset: 0,
        }
    }
}

impl MarkdownReportConfig {
    /// Create a minimal configuration (summary only).
    pub fn minimal() -> Self {
        Self {
            include_toc: false,
            include_summary: true,
            include_configuration: false,
            include_violations: true,
            include_coverage: false,
            include_recommendations: false,
            include_repro_bundle: false,
            max_violations_detail: 5,
            github_flavored: true,
            include_badges: true,
            heading_offset: 0,
        }
    }

    /// Create a full configuration (all sections).
    pub fn full() -> Self {
        Self {
            include_toc: true,
            include_summary: true,
            include_configuration: true,
            include_violations: true,
            include_coverage: true,
            include_recommendations: true,
            include_repro_bundle: true,
            max_violations_detail: 0,
            github_flavored: true,
            include_badges: true,
            heading_offset: 0,
        }
    }
}

/// Markdown report generator.
#[derive(Debug, Clone)]
pub struct MarkdownReportGenerator {
    config: MarkdownReportConfig,
}

impl MarkdownReportGenerator {
    /// Create a new Markdown report generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: MarkdownReportConfig::default(),
        }
    }

    /// Create a Markdown report generator with custom configuration.
    pub fn with_config(config: MarkdownReportConfig) -> Self {
        Self { config }
    }

    /// Generate Markdown report as a string.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id))]
    pub fn generate(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut md = String::with_capacity(16384);

        self.write_title(&mut md, report)?;

        if self.config.include_badges {
            self.write_badges(&mut md, report)?;
        }

        if self.config.include_toc {
            self.write_toc(&mut md, report)?;
        }

        if self.config.include_summary {
            self.write_summary_section(&mut md, report)?;
        }

        if self.config.include_configuration {
            self.write_configuration_section(&mut md, report)?;
        }

        if self.config.include_violations {
            self.write_violations_section(&mut md, report)?;
        }

        if self.config.include_coverage {
            self.write_coverage_section(&mut md, report)?;
        }

        if self.config.include_recommendations && !report.recommendations.is_empty() {
            self.write_recommendations_section(&mut md, report)?;
        }

        if self.config.include_repro_bundle && !report.repro_bundle.docker_compose.is_empty() {
            self.write_repro_bundle_section(&mut md, report)?;
        }

        self.write_footer(&mut md, report)?;

        Ok(md)
    }

    /// Generate Markdown report and write to a file.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id, path = %path.as_ref().display()))]
    pub fn generate_to_file(
        &self,
        report: &AuditReport,
        path: impl AsRef<Path>,
    ) -> Result<(), ReportError> {
        let md = self.generate(report)?;
        std::fs::write(path, md)?;
        Ok(())
    }

    /// Generate Markdown report and write to a writer.
    #[instrument(skip(self, report, writer), fields(report_id = %report.summary.report_id))]
    pub fn generate_to_writer<W: Write>(
        &self,
        report: &AuditReport,
        mut writer: W,
    ) -> Result<(), ReportError> {
        let md = self.generate(report)?;
        writer.write_all(md.as_bytes())?;
        Ok(())
    }

    fn heading(&self, level: usize) -> String {
        "#".repeat(level + self.config.heading_offset)
    }

    fn write_title(&self, md: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        md.push_str(&format!(
            "{} {}\n\n",
            self.heading(1),
            escape_markdown(&report.summary.title)
        ));
        Ok(())
    }

    fn write_badges(&self, md: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        // Status badge
        let (status_color, status_text) = if report.passed() {
            ("brightgreen", "PASSED")
        } else {
            ("red", "FAILED")
        };
        md.push_str(&format!(
            "![Status](https://img.shields.io/badge/status-{}-{})\n",
            status_text, status_color
        ));

        // Violations badge
        let violation_count = report.violations.len();
        let violation_color = if violation_count == 0 {
            "brightgreen"
        } else if violation_count < 5 {
            "yellow"
        } else {
            "red"
        };
        md.push_str(&format!(
            "![Violations](https://img.shields.io/badge/violations-{}-{})\n",
            violation_count, violation_color
        ));

        // Operations badge
        md.push_str(&format!(
            "![Operations](https://img.shields.io/badge/operations-{}-blue)\n",
            report.summary.total_operations
        ));

        md.push('\n');
        Ok(())
    }

    fn write_toc(&self, md: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        md.push_str(&format!("{} Table of Contents\n\n", self.heading(2)));

        md.push_str("- [Executive Summary](#executive-summary)\n");
        if self.config.include_configuration {
            md.push_str("- [Test Configuration](#test-configuration)\n");
        }
        if self.config.include_violations {
            md.push_str("- [Violations](#violations)\n");
        }
        if self.config.include_coverage {
            md.push_str("- [Test Coverage](#test-coverage)\n");
        }
        if self.config.include_recommendations && !report.recommendations.is_empty() {
            md.push_str("- [Recommendations](#recommendations)\n");
        }
        if self.config.include_repro_bundle && !report.repro_bundle.docker_compose.is_empty() {
            md.push_str("- [Reproducibility Bundle](#reproducibility-bundle)\n");
        }

        md.push('\n');
        Ok(())
    }

    fn write_summary_section(&self, md: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        md.push_str(&format!("{} Executive Summary\n\n", self.heading(2)));

        // Report info table
        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!(
            "| **Report ID** | `{}` |\n",
            report.summary.report_id
        ));
        md.push_str(&format!(
            "| **Test Run ID** | `{}` |\n",
            escape_markdown(&report.summary.test_run_id)
        ));
        md.push_str(&format!(
            "| **Generated** | {} |\n",
            report.summary.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!(
            "| **Duration** | {} |\n",
            format_duration(&report.summary.test_duration)
        ));
        md.push_str(&format!(
            "| **Status** | {} |\n",
            if report.passed() {
                "**PASSED**"
            } else {
                "**FAILED**"
            }
        ));
        md.push('\n');

        // Statistics
        md.push_str(&format!("{} Test Statistics\n\n", self.heading(3)));
        md.push_str(&format!(
            "- **Total Operations:** {}\n",
            report.summary.total_operations
        ));
        md.push_str(&format!(
            "- **Total Faults Injected:** {}\n",
            report.summary.total_faults
        ));
        md.push_str(&format!(
            "- **Violations Found:** {}\n",
            report.summary.total_violations
        ));

        // Violations by severity
        if !report.summary.violations_by_severity.is_empty() {
            md.push_str("\n**Violations by Severity:**\n\n");
            let mut severities: Vec<_> = report.summary.violations_by_severity.iter().collect();
            severities.sort_by(|a, b| severity_weight(b.0).cmp(&severity_weight(a.0)));

            for (severity, count) in severities {
                let emoji = severity_emoji(severity);
                md.push_str(&format!("- {} {}: {}\n", emoji, severity, count));
            }
        }

        md.push('\n');
        Ok(())
    }

    fn write_configuration_section(
        &self,
        md: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!("{} Test Configuration\n\n", self.heading(2)));

        md.push_str("| Setting | Value |\n");
        md.push_str("|---------|-------|\n");
        md.push_str(&format!(
            "| **Test Name** | {} |\n",
            escape_markdown(&report.test_configuration.name)
        ));
        md.push_str(&format!(
            "| **Consistency Model** | {} |\n",
            escape_markdown(&report.test_configuration.consistency_model)
        ));
        md.push_str(&format!(
            "| **System Under Test** | {} |\n",
            escape_markdown(&report.test_configuration.system_under_test)
        ));
        if let Some(ref version) = report.test_configuration.system_version {
            md.push_str(&format!(
                "| **System Version** | {} |\n",
                escape_markdown(version)
            ));
        }
        md.push_str(&format!(
            "| **Concurrency** | {} processes |\n",
            report.test_configuration.concurrency
        ));
        md.push_str(&format!(
            "| **Timeout** | {} |\n",
            format_duration(&report.test_configuration.timeout)
        ));
        if let Some(seed) = report.test_configuration.seed {
            md.push_str(&format!("| **Random Seed** | {} |\n", seed));
        }

        if let Some(ref desc) = report.test_configuration.description {
            md.push_str(&format!(
                "\n**Description:** {}\n",
                escape_markdown(desc)
            ));
        }

        md.push('\n');
        Ok(())
    }

    fn write_violations_section(
        &self,
        md: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!("{} Violations\n\n", self.heading(2)));

        if report.violations.is_empty() {
            md.push_str("> No violations detected. All consistency checks passed.\n\n");
            return Ok(());
        }

        md.push_str(&format!(
            "**{} violation(s) detected.**\n\n",
            report.violations.len()
        ));

        // Summary table
        md.push_str("| ID | Severity | Category | Description |\n");
        md.push_str("|----|----------|----------|-------------|\n");

        for violation in &report.violations {
            md.push_str(&format!(
                "| `{}` | {} {} | {} | {} |\n",
                &violation.id.to_string()[..8],
                severity_emoji(&violation.severity.to_string()),
                violation.severity,
                violation.category,
                truncate_string(&violation.description, 50)
            ));
        }

        md.push('\n');

        // Detailed violations
        let violations_to_show = if self.config.max_violations_detail == 0 {
            report.violations.len()
        } else {
            self.config.max_violations_detail.min(report.violations.len())
        };

        for violation in report.violations.iter().take(violations_to_show) {
            self.write_violation_detail(md, violation)?;
        }

        if violations_to_show < report.violations.len() {
            md.push_str(&format!(
                "\n*... and {} more violation(s) not shown.*\n\n",
                report.violations.len() - violations_to_show
            ));
        }

        Ok(())
    }

    fn write_violation_detail(
        &self,
        md: &mut String,
        violation: &ViolationReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!(
            "{} {} {}: {}\n\n",
            self.heading(3),
            severity_emoji(&violation.severity.to_string()),
            violation.severity,
            escape_markdown(&violation.category.to_string())
        ));

        md.push_str(&format!(
            "**ID:** `{}`\n\n",
            violation.id
        ));

        md.push_str(&format!(
            "**Description:** {}\n\n",
            escape_markdown(&violation.description)
        ));

        // Minimal history
        md.push_str(&format!(
            "**Minimal History:** Reduced from {} to {} operations\n\n",
            violation.minimal_history.original_size, violation.minimal_history.minimized_size
        ));

        if self.config.github_flavored {
            md.push_str("<details>\n<summary>Operation IDs</summary>\n\n");
            md.push_str("```\n");
            for op_id in &violation.minimal_history.operation_ids {
                md.push_str(&format!("{}\n", op_id));
            }
            md.push_str("```\n\n</details>\n\n");
        } else {
            md.push_str("**Operation IDs:**\n");
            for op_id in &violation.minimal_history.operation_ids {
                md.push_str(&format!("- `{}`\n", op_id));
            }
            md.push('\n');
        }

        // Root cause analysis
        if let Some(ref rca) = violation.root_cause_analysis {
            md.push_str(&format!("{} Root Cause Analysis\n\n", self.heading(4)));
            md.push_str(&format!(
                "**Category:** {}\n\n",
                escape_markdown(&rca.cause_category)
            ));
            md.push_str(&format!(
                "**Analysis:** {}\n\n",
                escape_markdown(&rca.analysis)
            ));
            md.push_str(&format!(
                "**Confidence:** {:.0}%\n\n",
                rca.confidence * 100.0
            ));

            if !rca.suggested_fixes.is_empty() {
                md.push_str("**Suggested Fixes:**\n");
                for fix in &rca.suggested_fixes {
                    md.push_str(&format!("- {}\n", escape_markdown(fix)));
                }
                md.push('\n');
            }
        }

        md.push_str("---\n\n");
        Ok(())
    }

    fn write_coverage_section(
        &self,
        md: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!("{} Test Coverage\n\n", self.heading(2)));

        md.push_str(&format!(
            "**State Space Coverage:** {:.1}%\n\n",
            report.coverage.state_space_coverage
        ));
        md.push_str(&format!(
            "**States Explored:** {}\n\n",
            report.coverage.states_explored
        ));

        // Operation coverage
        if !report.coverage.operation_coverage.is_empty() {
            md.push_str(&format!("{} Operation Coverage\n\n", self.heading(3)));
            md.push_str("| Operation | Executions | Success Rate |\n");
            md.push_str("|-----------|------------|-------------|\n");

            for (op_name, coverage) in &report.coverage.operation_coverage {
                md.push_str(&format!(
                    "| {} | {} | {:.1}% |\n",
                    escape_markdown(op_name),
                    coverage.execution_count,
                    coverage.success_rate * 100.0
                ));
            }
            md.push('\n');
        }

        // Fault coverage
        if !report.coverage.fault_coverage.is_empty() {
            md.push_str(&format!("{} Fault Coverage\n\n", self.heading(3)));
            md.push_str("| Fault Type | Injections | Recoveries |\n");
            md.push_str("|------------|------------|------------|\n");

            for (fault_type, coverage) in &report.coverage.fault_coverage {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    escape_markdown(fault_type),
                    coverage.injection_count,
                    coverage.successful_recoveries
                ));
            }
            md.push('\n');
        }

        Ok(())
    }

    fn write_recommendations_section(
        &self,
        md: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!("{} Recommendations\n\n", self.heading(2)));

        let mut sorted_recommendations = report.recommendations.clone();
        sorted_recommendations.sort_by_key(|r| r.priority);

        for rec in &sorted_recommendations {
            let priority_emoji = match rec.priority {
                1 => "",
                2 => "",
                3 => "",
                _ => "",
            };

            md.push_str(&format!(
                "{} {} Priority {}: {}\n\n",
                self.heading(3),
                priority_emoji,
                rec.priority,
                escape_markdown(&rec.title)
            ));

            md.push_str(&format!("{}\n\n", escape_markdown(&rec.description)));

            md.push_str(&format!(
                "**Expected Impact:** {}\n\n",
                escape_markdown(&rec.expected_impact)
            ));

            if !rec.action_items.is_empty() {
                md.push_str("**Action Items:**\n");
                for item in &rec.action_items {
                    md.push_str(&format!("- [ ] {}\n", escape_markdown(item)));
                }
                md.push('\n');
            }
        }

        Ok(())
    }

    fn write_repro_bundle_section(
        &self,
        md: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        md.push_str(&format!("{} Reproducibility Bundle\n\n", self.heading(2)));

        if !report.repro_bundle.instructions.is_empty() {
            md.push_str(&format!(
                "> {}\n\n",
                escape_markdown(&report.repro_bundle.instructions)
            ));
        }

        // Docker Compose
        md.push_str(&format!("{} docker-compose.yml\n\n", self.heading(3)));
        md.push_str("```yaml\n");
        md.push_str(&report.repro_bundle.docker_compose);
        md.push_str("\n```\n\n");

        // Test Script
        if !report.repro_bundle.test_script.is_empty() {
            md.push_str(&format!("{} Test Script\n\n", self.heading(3)));
            md.push_str("```bash\n");
            md.push_str(&report.repro_bundle.test_script);
            md.push_str("\n```\n\n");
        }

        // Expected Violation
        if !report.repro_bundle.expected_violation.is_empty() {
            md.push_str(&format!("{} Expected Violation\n\n", self.heading(3)));
            md.push_str(&format!(
                "{}\n\n",
                escape_markdown(&report.repro_bundle.expected_violation)
            ));
        }

        Ok(())
    }

    fn write_footer(&self, md: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        md.push_str("---\n\n");
        md.push_str(&format!(
            "*Generated by mamut-report v{} on {}*\n\n",
            env!("CARGO_PKG_VERSION"),
            report.summary.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!("*Report ID: `{}`*\n", report.summary.report_id));

        Ok(())
    }

    /// Generate a summary-only Markdown report.
    pub fn generate_summary(&self, report: &AuditReport) -> Result<String, ReportError> {
        let config = MarkdownReportConfig::minimal();
        let generator = MarkdownReportGenerator::with_config(config);
        generator.generate(report)
    }
}

impl Default for MarkdownReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape Markdown special characters.
fn escape_markdown(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('|', "\\|")
}

/// Format a duration for display.
fn format_duration(d: &std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs > 0 {
        format!("{}.{:03}s", secs, d.subsec_millis())
    } else {
        format!("{}ms", d.as_millis())
    }
}

/// Get severity weight for sorting.
fn severity_weight(s: &str) -> u8 {
    match s.to_uppercase().as_str() {
        "CRITICAL" => 5,
        "HIGH" => 4,
        "MEDIUM" => 3,
        "LOW" => 2,
        "INFO" => 1,
        _ => 0,
    }
}

/// Get emoji for severity level.
fn severity_emoji(s: &str) -> &'static str {
    match s.to_uppercase().as_str() {
        "CRITICAL" => "[X]",
        "HIGH" => "[!]",
        "MEDIUM" => "[~]",
        "LOW" => "[-]",
        "INFO" => "[i]",
        _ => "[ ]",
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        escape_markdown(s)
    } else {
        format!("{}...", escape_markdown(&s[..max_len - 3]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ReportSummary, TestConfiguration};
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
    fn test_markdown_generation() {
        let report = create_test_report();
        let generator = MarkdownReportGenerator::new();

        let md = generator.generate(&report).unwrap();

        assert!(md.contains("# Test Report"));
        assert!(md.contains("test-run-1"));
        assert!(md.contains("PASSED"));
    }

    #[test]
    fn test_markdown_escaping() {
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
        assert_eq!(escape_markdown("`code`"), "\\`code\\`");
        assert_eq!(escape_markdown("[link]"), "\\[link\\]");
    }

    #[test]
    fn test_markdown_minimal() {
        let report = create_test_report();
        let generator =
            MarkdownReportGenerator::with_config(MarkdownReportConfig::minimal());

        let md = generator.generate(&report).unwrap();

        assert!(md.contains("# Test Report"));
        assert!(!md.contains("Table of Contents"));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
    }
}
