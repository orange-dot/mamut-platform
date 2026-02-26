//! JSON report generation.
//!
//! This module provides JSON serialization for audit reports with
//! configurable formatting and streaming support for large reports.

use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde_json::ser::{CompactFormatter, PrettyFormatter, Serializer};
use tracing::instrument;

use crate::types::{AuditReport, ReportError, ViolationReport};

/// JSON report format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonFormat {
    /// Compact JSON (single line, minimal whitespace).
    Compact,
    /// Pretty-printed JSON with indentation.
    Pretty,
    /// Pretty-printed with custom indentation.
    PrettyIndent(usize),
}

impl Default for JsonFormat {
    fn default() -> Self {
        JsonFormat::Pretty
    }
}

/// Configuration for JSON report generation.
#[derive(Debug, Clone)]
pub struct JsonReportConfig {
    /// Output format.
    pub format: JsonFormat,
    /// Include null fields.
    pub include_nulls: bool,
    /// Include empty arrays.
    pub include_empty_arrays: bool,
    /// Maximum depth for nested objects (0 = unlimited).
    pub max_depth: usize,
    /// Include binary data (base64 encoded).
    pub include_binary_data: bool,
}

impl Default for JsonReportConfig {
    fn default() -> Self {
        Self {
            format: JsonFormat::Pretty,
            include_nulls: true,
            include_empty_arrays: true,
            max_depth: 0,
            include_binary_data: true,
        }
    }
}

impl JsonReportConfig {
    /// Create a compact configuration (smaller output).
    pub fn compact() -> Self {
        Self {
            format: JsonFormat::Compact,
            include_nulls: false,
            include_empty_arrays: false,
            max_depth: 0,
            include_binary_data: false,
        }
    }

    /// Create a configuration optimized for human readability.
    pub fn human_readable() -> Self {
        Self {
            format: JsonFormat::PrettyIndent(2),
            include_nulls: true,
            include_empty_arrays: true,
            max_depth: 0,
            include_binary_data: false,
        }
    }
}

/// JSON report generator.
#[derive(Debug, Clone)]
pub struct JsonReportGenerator {
    config: JsonReportConfig,
}

impl JsonReportGenerator {
    /// Create a new JSON report generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: JsonReportConfig::default(),
        }
    }

    /// Create a JSON report generator with custom configuration.
    pub fn with_config(config: JsonReportConfig) -> Self {
        Self { config }
    }

    /// Generate JSON report as a string.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id))]
    pub fn generate(&self, report: &AuditReport) -> Result<String, ReportError> {
        let json = match self.config.format {
            JsonFormat::Compact => serde_json::to_string(report)?,
            JsonFormat::Pretty => serde_json::to_string_pretty(report)?,
            JsonFormat::PrettyIndent(indent) => {
                let mut writer = Vec::new();
                let indent_str = " ".repeat(indent);
                let formatter = PrettyFormatter::with_indent(indent_str.as_bytes());
                let mut serializer = Serializer::with_formatter(&mut writer, formatter);
                report.serialize(&mut serializer)?;
                String::from_utf8(writer).map_err(|e| {
                    ReportError::SerializationError(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
            }
        };

        Ok(json)
    }

    /// Generate JSON report and write to a file.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id, path = %path.as_ref().display()))]
    pub fn generate_to_file(
        &self,
        report: &AuditReport,
        path: impl AsRef<Path>,
    ) -> Result<(), ReportError> {
        let json = self.generate(report)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Generate JSON report and write to a writer.
    #[instrument(skip(self, report, writer), fields(report_id = %report.summary.report_id))]
    pub fn generate_to_writer<W: Write>(
        &self,
        report: &AuditReport,
        mut writer: W,
    ) -> Result<(), ReportError> {
        match self.config.format {
            JsonFormat::Compact => {
                let formatter = CompactFormatter;
                let mut serializer = Serializer::with_formatter(&mut writer, formatter);
                report.serialize(&mut serializer)?;
            }
            JsonFormat::Pretty => {
                let formatter = PrettyFormatter::new();
                let mut serializer = Serializer::with_formatter(&mut writer, formatter);
                report.serialize(&mut serializer)?;
            }
            JsonFormat::PrettyIndent(indent) => {
                let indent_str = " ".repeat(indent);
                let formatter = PrettyFormatter::with_indent(indent_str.as_bytes());
                let mut serializer = Serializer::with_formatter(&mut writer, formatter);
                report.serialize(&mut serializer)?;
            }
        }
        Ok(())
    }

    /// Generate a summary-only JSON report (without full violation details).
    pub fn generate_summary(&self, report: &AuditReport) -> Result<String, ReportError> {
        let summary = JsonSummary {
            summary: &report.summary,
            test_configuration: &report.test_configuration,
            violation_count: report.violations.len(),
            violation_ids: report.violations.iter().map(|v| v.id.to_string()).collect(),
            passed: report.passed(),
        };

        let json = match self.config.format {
            JsonFormat::Compact => serde_json::to_string(&summary)?,
            _ => serde_json::to_string_pretty(&summary)?,
        };

        Ok(json)
    }

    /// Generate JSON for a single violation.
    pub fn generate_violation(&self, violation: &ViolationReport) -> Result<String, ReportError> {
        let json = match self.config.format {
            JsonFormat::Compact => serde_json::to_string(violation)?,
            _ => serde_json::to_string_pretty(violation)?,
        };

        Ok(json)
    }
}

impl Default for JsonReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary-only view of a report for lightweight JSON output.
#[derive(Serialize)]
struct JsonSummary<'a> {
    summary: &'a crate::types::ReportSummary,
    test_configuration: &'a crate::types::TestConfiguration,
    violation_count: usize,
    violation_ids: Vec<String>,
    passed: bool,
}

/// JSON Lines (JSONL) report generator for streaming large reports.
#[derive(Debug, Clone)]
pub struct JsonLinesGenerator {
    /// Whether to include a header line.
    include_header: bool,
}

impl JsonLinesGenerator {
    /// Create a new JSON Lines generator.
    pub fn new() -> Self {
        Self {
            include_header: true,
        }
    }

    /// Set whether to include a header line.
    pub fn with_header(mut self, include: bool) -> Self {
        self.include_header = include;
        self
    }

    /// Generate JSON Lines report (one JSON object per line).
    pub fn generate(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut lines = Vec::new();

        if self.include_header {
            let header = JsonLHeader {
                report_id: report.summary.report_id.to_string(),
                generated_at: report.summary.generated_at,
                record_type: "header".to_string(),
            };
            lines.push(serde_json::to_string(&header)?);
        }

        // Add summary
        let summary_line = JsonLRecord {
            record_type: "summary".to_string(),
            data: serde_json::to_value(&report.summary)?,
        };
        lines.push(serde_json::to_string(&summary_line)?);

        // Add test configuration
        let config_line = JsonLRecord {
            record_type: "configuration".to_string(),
            data: serde_json::to_value(&report.test_configuration)?,
        };
        lines.push(serde_json::to_string(&config_line)?);

        // Add history analysis
        let analysis_line = JsonLRecord {
            record_type: "history_analysis".to_string(),
            data: serde_json::to_value(&report.history_analysis)?,
        };
        lines.push(serde_json::to_string(&analysis_line)?);

        // Add each violation as a separate line
        for violation in &report.violations {
            let violation_line = JsonLRecord {
                record_type: "violation".to_string(),
                data: serde_json::to_value(violation)?,
            };
            lines.push(serde_json::to_string(&violation_line)?);
        }

        // Add coverage
        let coverage_line = JsonLRecord {
            record_type: "coverage".to_string(),
            data: serde_json::to_value(&report.coverage)?,
        };
        lines.push(serde_json::to_string(&coverage_line)?);

        // Add recommendations
        for recommendation in &report.recommendations {
            let rec_line = JsonLRecord {
                record_type: "recommendation".to_string(),
                data: serde_json::to_value(recommendation)?,
            };
            lines.push(serde_json::to_string(&rec_line)?);
        }

        Ok(lines.join("\n"))
    }

    /// Stream JSON Lines to a writer.
    pub fn stream_to_writer<W: Write>(
        &self,
        report: &AuditReport,
        mut writer: W,
    ) -> Result<(), ReportError> {
        if self.include_header {
            let header = JsonLHeader {
                report_id: report.summary.report_id.to_string(),
                generated_at: report.summary.generated_at,
                record_type: "header".to_string(),
            };
            writeln!(writer, "{}", serde_json::to_string(&header)?)?;
        }

        // Stream summary
        let summary_line = JsonLRecord {
            record_type: "summary".to_string(),
            data: serde_json::to_value(&report.summary)?,
        };
        writeln!(writer, "{}", serde_json::to_string(&summary_line)?)?;

        // Stream configuration
        let config_line = JsonLRecord {
            record_type: "configuration".to_string(),
            data: serde_json::to_value(&report.test_configuration)?,
        };
        writeln!(writer, "{}", serde_json::to_string(&config_line)?)?;

        // Stream history analysis
        let analysis_line = JsonLRecord {
            record_type: "history_analysis".to_string(),
            data: serde_json::to_value(&report.history_analysis)?,
        };
        writeln!(writer, "{}", serde_json::to_string(&analysis_line)?)?;

        // Stream violations
        for violation in &report.violations {
            let violation_line = JsonLRecord {
                record_type: "violation".to_string(),
                data: serde_json::to_value(violation)?,
            };
            writeln!(writer, "{}", serde_json::to_string(&violation_line)?)?;
        }

        // Stream coverage
        let coverage_line = JsonLRecord {
            record_type: "coverage".to_string(),
            data: serde_json::to_value(&report.coverage)?,
        };
        writeln!(writer, "{}", serde_json::to_string(&coverage_line)?)?;

        // Stream recommendations
        for recommendation in &report.recommendations {
            let rec_line = JsonLRecord {
                record_type: "recommendation".to_string(),
                data: serde_json::to_value(recommendation)?,
            };
            writeln!(writer, "{}", serde_json::to_string(&rec_line)?)?;
        }

        Ok(())
    }
}

impl Default for JsonLinesGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Header record for JSON Lines format.
#[derive(Serialize)]
struct JsonLHeader {
    record_type: String,
    report_id: String,
    generated_at: chrono::DateTime<chrono::Utc>,
}

/// Generic record for JSON Lines format.
#[derive(Serialize)]
struct JsonLRecord {
    record_type: String,
    data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ReportSummary, TestConfiguration,
    };
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_report() -> AuditReport {
        let summary = ReportSummary::new("test-run-1".to_string(), "Test Report".to_string());
        let config = TestConfiguration {
            name: "test".to_string(),
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

        AuditReport::new(summary, config)
    }

    #[test]
    fn test_json_generation_compact() {
        let report = create_test_report();
        let generator = JsonReportGenerator::with_config(JsonReportConfig::compact());

        let json = generator.generate(&report).unwrap();

        assert!(!json.contains('\n'));
        assert!(json.contains("test-run-1"));
    }

    #[test]
    fn test_json_generation_pretty() {
        let report = create_test_report();
        let generator = JsonReportGenerator::new();

        let json = generator.generate(&report).unwrap();

        assert!(json.contains('\n'));
        assert!(json.contains("test-run-1"));
    }

    #[test]
    fn test_json_summary_generation() {
        let report = create_test_report();
        let generator = JsonReportGenerator::new();

        let json = generator.generate_summary(&report).unwrap();

        assert!(json.contains("test-run-1"));
        assert!(json.contains("\"passed\""));
    }

    #[test]
    fn test_jsonl_generation() {
        let report = create_test_report();
        let generator = JsonLinesGenerator::new();

        let jsonl = generator.generate(&report).unwrap();
        let lines: Vec<_> = jsonl.lines().collect();

        assert!(lines.len() >= 4); // header, summary, configuration, analysis
        assert!(lines[0].contains("header"));
    }

    #[test]
    fn test_json_to_writer() {
        let report = create_test_report();
        let generator = JsonReportGenerator::new();

        let mut buffer = Vec::new();
        generator.generate_to_writer(&report, &mut buffer).unwrap();

        let json = String::from_utf8(buffer).unwrap();
        assert!(json.contains("test-run-1"));
    }
}
