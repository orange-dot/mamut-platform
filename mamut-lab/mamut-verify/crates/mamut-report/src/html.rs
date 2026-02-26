//! HTML report generation with timeline visualization.
//!
//! This module provides HTML report generation with embedded CSS for
//! audit-grade documentation with interactive timeline visualization.

use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use tracing::instrument;

use crate::timeline::{TimelineData, TimelineEventType, TimelineGenerator};
use crate::types::{AuditReport, ReportError, ViolationReport};

/// Configuration for HTML report generation.
#[derive(Debug, Clone)]
pub struct HtmlReportConfig {
    /// Include embedded CSS (default: true).
    pub embed_css: bool,
    /// Include embedded JavaScript for interactivity (default: true).
    pub embed_js: bool,
    /// Include timeline visualization (default: true).
    pub include_timeline: bool,
    /// Include repro bundle section (default: true).
    pub include_repro_bundle: bool,
    /// Custom page title (default: report title).
    pub page_title: Option<String>,
    /// Custom CSS to inject.
    pub custom_css: Option<String>,
    /// Custom JavaScript to inject.
    pub custom_js: Option<String>,
    /// Company/organization name for branding.
    pub organization_name: Option<String>,
    /// Logo URL for branding.
    pub logo_url: Option<String>,
}

impl Default for HtmlReportConfig {
    fn default() -> Self {
        Self {
            embed_css: true,
            embed_js: true,
            include_timeline: true,
            include_repro_bundle: true,
            page_title: None,
            custom_css: None,
            custom_js: None,
            organization_name: None,
            logo_url: None,
        }
    }
}

/// HTML report generator.
#[derive(Debug, Clone)]
pub struct HtmlReportGenerator {
    config: HtmlReportConfig,
}

impl HtmlReportGenerator {
    /// Create a new HTML report generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: HtmlReportConfig::default(),
        }
    }

    /// Create an HTML report generator with custom configuration.
    pub fn with_config(config: HtmlReportConfig) -> Self {
        Self { config }
    }

    /// Generate HTML report as a string.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id))]
    pub fn generate(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut html = String::with_capacity(65536);

        self.write_doctype(&mut html);
        self.write_head(&mut html, report)?;
        self.write_body(&mut html, report)?;

        Ok(html)
    }

    /// Generate HTML report and write to a file.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id, path = %path.as_ref().display()))]
    pub fn generate_to_file(
        &self,
        report: &AuditReport,
        path: impl AsRef<Path>,
    ) -> Result<(), ReportError> {
        let html = self.generate(report)?;
        std::fs::write(path, html)?;
        Ok(())
    }

    /// Generate HTML report and write to a writer.
    #[instrument(skip(self, report, writer), fields(report_id = %report.summary.report_id))]
    pub fn generate_to_writer<W: Write>(
        &self,
        report: &AuditReport,
        mut writer: W,
    ) -> Result<(), ReportError> {
        let html = self.generate(report)?;
        writer.write_all(html.as_bytes())?;
        Ok(())
    }

    fn write_doctype(&self, html: &mut String) {
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n");
    }

    fn write_head(&self, html: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        let title = self
            .config
            .page_title
            .as_ref()
            .unwrap_or(&report.summary.title);

        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str(
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
        );
        html.push_str(&format!("  <title>{}</title>\n", escape_html(title)));
        html.push_str(&format!(
            "  <meta name=\"generator\" content=\"mamut-report {}\">\n",
            env!("CARGO_PKG_VERSION")
        ));
        html.push_str(&format!(
            "  <meta name=\"generated-at\" content=\"{}\">\n",
            report.summary.generated_at.to_rfc3339()
        ));

        if self.config.embed_css {
            html.push_str("  <style>\n");
            html.push_str(EMBEDDED_CSS);
            html.push_str("  </style>\n");
        }

        if let Some(ref custom_css) = self.config.custom_css {
            html.push_str("  <style>\n");
            html.push_str(custom_css);
            html.push_str("  </style>\n");
        }

        html.push_str("</head>\n");
        Ok(())
    }

    fn write_body(&self, html: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        html.push_str("<body>\n");

        self.write_header(html, report)?;
        self.write_summary_section(html, report)?;
        self.write_configuration_section(html, report)?;

        if self.config.include_timeline {
            self.write_timeline_section(html, report)?;
        }

        self.write_violations_section(html, report)?;
        self.write_coverage_section(html, report)?;
        self.write_recommendations_section(html, report)?;

        if self.config.include_repro_bundle && !report.repro_bundle.docker_compose.is_empty() {
            self.write_repro_bundle_section(html, report)?;
        }

        self.write_footer(html, report)?;

        if self.config.embed_js {
            html.push_str("<script>\n");
            html.push_str(EMBEDDED_JS);
            html.push_str("</script>\n");
        }

        if let Some(ref custom_js) = self.config.custom_js {
            html.push_str("<script>\n");
            html.push_str(custom_js);
            html.push_str("</script>\n");
        }

        html.push_str("</body>\n</html>\n");
        Ok(())
    }

    fn write_header(&self, html: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        html.push_str("<header class=\"report-header\">\n");

        if let Some(ref logo_url) = self.config.logo_url {
            html.push_str(&format!(
                "  <img src=\"{}\" alt=\"Logo\" class=\"logo\">\n",
                escape_html(logo_url)
            ));
        }

        html.push_str(&format!(
            "  <h1>{}</h1>\n",
            escape_html(&report.summary.title)
        ));

        if let Some(ref org) = self.config.organization_name {
            html.push_str(&format!(
                "  <p class=\"organization\">{}</p>\n",
                escape_html(org)
            ));
        }

        let status_class = if report.passed() { "passed" } else { "failed" };
        let status_text = if report.passed() { "PASSED" } else { "FAILED" };
        html.push_str(&format!(
            "  <div class=\"status-badge {}\">{}</div>\n",
            status_class, status_text
        ));

        html.push_str("</header>\n\n");
        Ok(())
    }

    fn write_summary_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        html.push_str("<section class=\"summary-section\">\n");
        html.push_str("  <h2>Executive Summary</h2>\n");

        html.push_str("  <div class=\"summary-grid\">\n");

        // Report metadata
        html.push_str("    <div class=\"summary-card\">\n");
        html.push_str("      <h3>Report Information</h3>\n");
        html.push_str("      <dl>\n");
        html.push_str(&format!(
            "        <dt>Report ID</dt><dd><code>{}</code></dd>\n",
            report.summary.report_id
        ));
        html.push_str(&format!(
            "        <dt>Test Run ID</dt><dd><code>{}</code></dd>\n",
            escape_html(&report.summary.test_run_id)
        ));
        html.push_str(&format!(
            "        <dt>Generated</dt><dd>{}</dd>\n",
            format_datetime(&report.summary.generated_at)
        ));
        html.push_str(&format!(
            "        <dt>Duration</dt><dd>{}</dd>\n",
            format_duration(&report.summary.test_duration)
        ));
        html.push_str("      </dl>\n");
        html.push_str("    </div>\n");

        // Test statistics
        html.push_str("    <div class=\"summary-card\">\n");
        html.push_str("      <h3>Test Statistics</h3>\n");
        html.push_str("      <dl>\n");
        html.push_str(&format!(
            "        <dt>Total Operations</dt><dd>{}</dd>\n",
            report.summary.total_operations
        ));
        html.push_str(&format!(
            "        <dt>Total Faults</dt><dd>{}</dd>\n",
            report.summary.total_faults
        ));
        html.push_str(&format!(
            "        <dt>Violations Found</dt><dd class=\"{}\">{}</dd>\n",
            if report.summary.total_violations > 0 {
                "violation-count"
            } else {
                ""
            },
            report.summary.total_violations
        ));
        html.push_str("      </dl>\n");
        html.push_str("    </div>\n");

        // Violations by severity
        if !report.summary.violations_by_severity.is_empty() {
            html.push_str("    <div class=\"summary-card\">\n");
            html.push_str("      <h3>Violations by Severity</h3>\n");
            html.push_str("      <ul class=\"severity-list\">\n");

            let mut severities: Vec<_> = report.summary.violations_by_severity.iter().collect();
            severities.sort_by(|a, b| {
                severity_weight(b.0).cmp(&severity_weight(a.0))
            });

            for (severity, count) in severities {
                let css_class = severity_css_class(severity);
                html.push_str(&format!(
                    "        <li class=\"{}\"><span class=\"severity-label\">{}</span>: {}</li>\n",
                    css_class, severity, count
                ));
            }
            html.push_str("      </ul>\n");
            html.push_str("    </div>\n");
        }

        html.push_str("  </div>\n");
        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_configuration_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        html.push_str("<section class=\"configuration-section\">\n");
        html.push_str("  <h2>Test Configuration</h2>\n");

        html.push_str("  <div class=\"config-grid\">\n");

        html.push_str("    <div class=\"config-card\">\n");
        html.push_str("      <h3>Test Details</h3>\n");
        html.push_str("      <dl>\n");
        html.push_str(&format!(
            "        <dt>Test Name</dt><dd>{}</dd>\n",
            escape_html(&report.test_configuration.name)
        ));
        if let Some(ref desc) = report.test_configuration.description {
            html.push_str(&format!(
                "        <dt>Description</dt><dd>{}</dd>\n",
                escape_html(desc)
            ));
        }
        html.push_str(&format!(
            "        <dt>Consistency Model</dt><dd>{}</dd>\n",
            escape_html(&report.test_configuration.consistency_model)
        ));
        html.push_str(&format!(
            "        <dt>Concurrency</dt><dd>{} processes</dd>\n",
            report.test_configuration.concurrency
        ));
        html.push_str(&format!(
            "        <dt>Timeout</dt><dd>{}</dd>\n",
            format_duration(&report.test_configuration.timeout)
        ));
        if let Some(seed) = report.test_configuration.seed {
            html.push_str(&format!("        <dt>Random Seed</dt><dd>{}</dd>\n", seed));
        }
        html.push_str("      </dl>\n");
        html.push_str("    </div>\n");

        html.push_str("    <div class=\"config-card\">\n");
        html.push_str("      <h3>System Under Test</h3>\n");
        html.push_str("      <dl>\n");
        html.push_str(&format!(
            "        <dt>System</dt><dd>{}</dd>\n",
            escape_html(&report.test_configuration.system_under_test)
        ));
        if let Some(ref version) = report.test_configuration.system_version {
            html.push_str(&format!(
                "        <dt>Version</dt><dd>{}</dd>\n",
                escape_html(version)
            ));
        }
        html.push_str("      </dl>\n");
        html.push_str("    </div>\n");

        html.push_str("  </div>\n");
        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_timeline_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        let timeline_generator = TimelineGenerator::new();
        let timeline_data = timeline_generator.generate_from_report(report);

        html.push_str("<section class=\"timeline-section\">\n");
        html.push_str("  <h2>Execution Timeline</h2>\n");

        if timeline_data.events.is_empty() {
            html.push_str("  <p class=\"no-data\">No timeline events available.</p>\n");
        } else {
            html.push_str("  <div class=\"timeline-container\" id=\"timeline\">\n");
            self.write_timeline_visualization(html, &timeline_data)?;
            html.push_str("  </div>\n");

            // Timeline data for JavaScript
            html.push_str("  <script type=\"application/json\" id=\"timeline-data\">\n");
            html.push_str(&serde_json::to_string(&timeline_data).unwrap_or_default());
            html.push_str("  </script>\n");
        }

        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_timeline_visualization(
        &self,
        html: &mut String,
        timeline_data: &TimelineData,
    ) -> Result<(), ReportError> {
        html.push_str("    <div class=\"timeline\">\n");

        for event in &timeline_data.events {
            let event_class = match event.event_type {
                TimelineEventType::OperationStart => "timeline-event operation-start",
                TimelineEventType::OperationEnd => "timeline-event operation-end",
                TimelineEventType::FaultInjected => "timeline-event fault-injected",
                TimelineEventType::FaultHealed => "timeline-event fault-healed",
                TimelineEventType::ViolationDetected => "timeline-event violation-detected",
                TimelineEventType::CheckStarted => "timeline-event check-started",
                TimelineEventType::CheckCompleted => "timeline-event check-completed",
                TimelineEventType::PhaseTransition => "timeline-event phase-transition",
                TimelineEventType::Custom(_) => "timeline-event custom",
            };

            html.push_str(&format!(
                "      <div class=\"{}\" data-timestamp=\"{}\" data-id=\"{}\">\n",
                event_class,
                event.timestamp.timestamp_millis(),
                escape_html(&event.id)
            ));
            html.push_str(&format!(
                "        <span class=\"timeline-time\">{}</span>\n",
                event.timestamp.format("%H:%M:%S%.3f")
            ));
            html.push_str(&format!(
                "        <span class=\"timeline-label\">{}</span>\n",
                escape_html(&event.label)
            ));
            if let Some(ref desc) = event.description {
                html.push_str(&format!(
                    "        <span class=\"timeline-description\">{}</span>\n",
                    escape_html(desc)
                ));
            }
            html.push_str("      </div>\n");
        }

        html.push_str("    </div>\n");
        Ok(())
    }

    fn write_violations_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        html.push_str("<section class=\"violations-section\">\n");
        html.push_str("  <h2>Violations</h2>\n");

        if report.violations.is_empty() {
            html.push_str(
                "  <p class=\"no-violations\">No violations detected. All checks passed.</p>\n",
            );
        } else {
            html.push_str(&format!(
                "  <p class=\"violation-summary\">{} violation(s) detected.</p>\n",
                report.violations.len()
            ));

            // Sort by severity
            let sorted_violations = report.violations_by_severity();

            for violation in sorted_violations {
                self.write_violation_card(html, violation)?;
            }
        }

        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_violation_card(
        &self,
        html: &mut String,
        violation: &ViolationReport,
    ) -> Result<(), ReportError> {
        html.push_str(&format!(
            "  <div class=\"violation-card {}\" id=\"violation-{}\">\n",
            violation.severity.css_class(),
            violation.id
        ));

        // Header
        html.push_str("    <div class=\"violation-header\">\n");
        html.push_str(&format!(
            "      <span class=\"severity-badge {}\">{}</span>\n",
            violation.severity.css_class(),
            violation.severity
        ));
        html.push_str(&format!(
            "      <span class=\"violation-category\">{}</span>\n",
            escape_html(&violation.category.to_string())
        ));
        html.push_str(&format!(
            "      <span class=\"violation-id\">ID: {}</span>\n",
            violation.id
        ));
        html.push_str("    </div>\n");

        // Description
        html.push_str("    <div class=\"violation-body\">\n");
        html.push_str(&format!(
            "      <p class=\"violation-description\">{}</p>\n",
            escape_html(&violation.description)
        ));

        // Minimal history
        html.push_str("      <div class=\"minimal-history\">\n");
        html.push_str("        <h4>Minimal History</h4>\n");
        html.push_str(&format!(
            "        <p>Reduced from {} to {} operations</p>\n",
            violation.minimal_history.original_size, violation.minimal_history.minimized_size
        ));
        html.push_str("        <ul class=\"operation-list\">\n");
        for op_id in &violation.minimal_history.operation_ids {
            html.push_str(&format!(
                "          <li><code>{}</code></li>\n",
                escape_html(op_id)
            ));
        }
        html.push_str("        </ul>\n");
        html.push_str("      </div>\n");

        // Root cause analysis
        if let Some(ref rca) = violation.root_cause_analysis {
            html.push_str("      <div class=\"root-cause-analysis\">\n");
            html.push_str("        <h4>Root Cause Analysis</h4>\n");
            html.push_str(&format!(
                "        <p class=\"cause-category\"><strong>Category:</strong> {}</p>\n",
                escape_html(&rca.cause_category)
            ));
            html.push_str(&format!(
                "        <p class=\"analysis\">{}</p>\n",
                escape_html(&rca.analysis)
            ));
            html.push_str(&format!(
                "        <p class=\"confidence\"><strong>Confidence:</strong> {:.0}%</p>\n",
                rca.confidence * 100.0
            ));

            if !rca.suggested_fixes.is_empty() {
                html.push_str("        <h5>Suggested Fixes</h5>\n");
                html.push_str("        <ul>\n");
                for fix in &rca.suggested_fixes {
                    html.push_str(&format!("          <li>{}</li>\n", escape_html(fix)));
                }
                html.push_str("        </ul>\n");
            }

            html.push_str("      </div>\n");
        }

        // Evidence
        if !violation.evidence.is_empty() {
            html.push_str("      <div class=\"evidence\">\n");
            html.push_str("        <h4>Evidence</h4>\n");
            html.push_str("        <ul class=\"evidence-list\">\n");
            for evidence in &violation.evidence {
                html.push_str(&format!(
                    "          <li><strong>{:?}:</strong> {}</li>\n",
                    evidence.evidence_type,
                    escape_html(&evidence.description)
                ));
            }
            html.push_str("        </ul>\n");
            html.push_str("      </div>\n");
        }

        html.push_str("    </div>\n");
        html.push_str("  </div>\n\n");
        Ok(())
    }

    fn write_coverage_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        html.push_str("<section class=\"coverage-section\">\n");
        html.push_str("  <h2>Test Coverage</h2>\n");

        html.push_str("  <div class=\"coverage-grid\">\n");

        // State space coverage
        html.push_str("    <div class=\"coverage-card\">\n");
        html.push_str("      <h3>State Space Coverage</h3>\n");
        html.push_str(&format!(
            "      <div class=\"coverage-percentage\">{:.1}%</div>\n",
            report.coverage.state_space_coverage
        ));
        html.push_str(&format!(
            "      <p>States explored: {}</p>\n",
            report.coverage.states_explored
        ));
        if let Some(total) = report.coverage.estimated_state_space {
            html.push_str(&format!("      <p>Estimated total: {}</p>\n", total));
        }
        html.push_str("    </div>\n");

        // Operation coverage
        if !report.coverage.operation_coverage.is_empty() {
            html.push_str("    <div class=\"coverage-card\">\n");
            html.push_str("      <h3>Operation Coverage</h3>\n");
            html.push_str("      <table class=\"coverage-table\">\n");
            html.push_str("        <tr><th>Operation</th><th>Count</th><th>Success Rate</th></tr>\n");
            for (op_name, coverage) in &report.coverage.operation_coverage {
                html.push_str(&format!(
                    "        <tr><td>{}</td><td>{}</td><td>{:.1}%</td></tr>\n",
                    escape_html(op_name),
                    coverage.execution_count,
                    coverage.success_rate * 100.0
                ));
            }
            html.push_str("      </table>\n");
            html.push_str("    </div>\n");
        }

        // Fault coverage
        if !report.coverage.fault_coverage.is_empty() {
            html.push_str("    <div class=\"coverage-card\">\n");
            html.push_str("      <h3>Fault Coverage</h3>\n");
            html.push_str("      <table class=\"coverage-table\">\n");
            html.push_str(
                "        <tr><th>Fault Type</th><th>Injected</th><th>Recoveries</th></tr>\n",
            );
            for (fault_type, coverage) in &report.coverage.fault_coverage {
                html.push_str(&format!(
                    "        <tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    escape_html(fault_type),
                    coverage.injection_count,
                    coverage.successful_recoveries
                ));
            }
            html.push_str("      </table>\n");
            html.push_str("    </div>\n");
        }

        html.push_str("  </div>\n");
        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_recommendations_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        if report.recommendations.is_empty() {
            return Ok(());
        }

        html.push_str("<section class=\"recommendations-section\">\n");
        html.push_str("  <h2>Recommendations</h2>\n");

        let mut sorted_recommendations = report.recommendations.clone();
        sorted_recommendations.sort_by_key(|r| r.priority);

        for rec in &sorted_recommendations {
            html.push_str(&format!(
                "  <div class=\"recommendation-card priority-{}\">\n",
                rec.priority
            ));
            html.push_str(&format!(
                "    <div class=\"recommendation-header\">\n      <span class=\"priority\">Priority {}</span>\n      <h3>{}</h3>\n    </div>\n",
                rec.priority,
                escape_html(&rec.title)
            ));
            html.push_str(&format!(
                "    <p class=\"recommendation-description\">{}</p>\n",
                escape_html(&rec.description)
            ));
            html.push_str(&format!(
                "    <p class=\"expected-impact\"><strong>Expected Impact:</strong> {}</p>\n",
                escape_html(&rec.expected_impact)
            ));

            if !rec.action_items.is_empty() {
                html.push_str("    <h4>Action Items</h4>\n");
                html.push_str("    <ul>\n");
                for item in &rec.action_items {
                    html.push_str(&format!("      <li>{}</li>\n", escape_html(item)));
                }
                html.push_str("    </ul>\n");
            }

            html.push_str("  </div>\n");
        }

        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_repro_bundle_section(
        &self,
        html: &mut String,
        report: &AuditReport,
    ) -> Result<(), ReportError> {
        html.push_str("<section class=\"repro-bundle-section\">\n");
        html.push_str("  <h2>Reproducibility Bundle</h2>\n");

        html.push_str("  <p class=\"repro-instructions\">");
        html.push_str(&escape_html(&report.repro_bundle.instructions));
        html.push_str("</p>\n");

        // Docker Compose
        html.push_str("  <div class=\"code-block\">\n");
        html.push_str("    <h3>docker-compose.yml</h3>\n");
        html.push_str("    <pre><code class=\"yaml\">");
        html.push_str(&escape_html(&report.repro_bundle.docker_compose));
        html.push_str("</code></pre>\n");
        html.push_str("  </div>\n");

        // Test Script
        if !report.repro_bundle.test_script.is_empty() {
            html.push_str("  <div class=\"code-block\">\n");
            html.push_str("    <h3>Test Script</h3>\n");
            html.push_str("    <pre><code class=\"bash\">");
            html.push_str(&escape_html(&report.repro_bundle.test_script));
            html.push_str("</code></pre>\n");
            html.push_str("  </div>\n");
        }

        // Expected Violation
        if !report.repro_bundle.expected_violation.is_empty() {
            html.push_str("  <div class=\"expected-violation\">\n");
            html.push_str("    <h3>Expected Violation</h3>\n");
            html.push_str(&format!(
                "    <p>{}</p>\n",
                escape_html(&report.repro_bundle.expected_violation)
            ));
            html.push_str("  </div>\n");
        }

        html.push_str("</section>\n\n");
        Ok(())
    }

    fn write_footer(&self, html: &mut String, report: &AuditReport) -> Result<(), ReportError> {
        html.push_str("<footer class=\"report-footer\">\n");
        html.push_str(&format!(
            "  <p>Generated by mamut-report v{} on {}</p>\n",
            env!("CARGO_PKG_VERSION"),
            report.summary.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        html.push_str(&format!(
            "  <p>Report ID: {}</p>\n",
            report.summary.report_id
        ));
        html.push_str("</footer>\n");
        Ok(())
    }
}

impl Default for HtmlReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Format a datetime for display.
fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
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

/// Get CSS class for severity.
fn severity_css_class(s: &str) -> &'static str {
    match s.to_uppercase().as_str() {
        "CRITICAL" => "severity-critical",
        "HIGH" => "severity-high",
        "MEDIUM" => "severity-medium",
        "LOW" => "severity-low",
        "INFO" => "severity-info",
        _ => "",
    }
}

/// Embedded CSS for the HTML report.
const EMBEDDED_CSS: &str = r#"
:root {
  --color-critical: #dc2626;
  --color-high: #ea580c;
  --color-medium: #ca8a04;
  --color-low: #2563eb;
  --color-info: #6b7280;
  --color-pass: #16a34a;
  --color-fail: #dc2626;
  --color-bg: #ffffff;
  --color-bg-secondary: #f3f4f6;
  --color-text: #1f2937;
  --color-text-secondary: #6b7280;
  --color-border: #e5e7eb;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  line-height: 1.6;
  color: var(--color-text);
  background-color: var(--color-bg);
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}

h1, h2, h3, h4 {
  margin-bottom: 1rem;
  font-weight: 600;
}

h1 { font-size: 2rem; }
h2 { font-size: 1.5rem; border-bottom: 2px solid var(--color-border); padding-bottom: 0.5rem; margin-top: 2rem; }
h3 { font-size: 1.25rem; }
h4 { font-size: 1rem; }

p { margin-bottom: 1rem; }

code {
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 0.875rem;
  background-color: var(--color-bg-secondary);
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
}

pre {
  background-color: var(--color-bg-secondary);
  padding: 1rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  margin-bottom: 1rem;
}

pre code {
  background: none;
  padding: 0;
}

/* Header */
.report-header {
  text-align: center;
  margin-bottom: 2rem;
  padding-bottom: 2rem;
  border-bottom: 2px solid var(--color-border);
}

.report-header .logo {
  max-height: 60px;
  margin-bottom: 1rem;
}

.report-header .organization {
  color: var(--color-text-secondary);
}

.status-badge {
  display: inline-block;
  padding: 0.5rem 1.5rem;
  border-radius: 9999px;
  font-weight: bold;
  font-size: 1.25rem;
  margin-top: 1rem;
}

.status-badge.passed {
  background-color: var(--color-pass);
  color: white;
}

.status-badge.failed {
  background-color: var(--color-fail);
  color: white;
}

/* Summary Grid */
.summary-grid, .config-grid, .coverage-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1.5rem;
  margin-top: 1rem;
}

.summary-card, .config-card, .coverage-card {
  background-color: var(--color-bg-secondary);
  padding: 1.5rem;
  border-radius: 0.5rem;
  border: 1px solid var(--color-border);
}

dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.5rem 1rem;
}

dt {
  font-weight: 600;
  color: var(--color-text-secondary);
}

dd {
  margin: 0;
}

.violation-count {
  color: var(--color-fail);
  font-weight: bold;
}

/* Severity */
.severity-list {
  list-style: none;
}

.severity-list li {
  padding: 0.25rem 0;
}

.severity-label {
  font-weight: 600;
}

.severity-critical { color: var(--color-critical); }
.severity-high { color: var(--color-high); }
.severity-medium { color: var(--color-medium); }
.severity-low { color: var(--color-low); }
.severity-info { color: var(--color-info); }

.severity-badge {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  border-radius: 0.25rem;
  font-weight: bold;
  font-size: 0.75rem;
  text-transform: uppercase;
  color: white;
}

.severity-badge.severity-critical { background-color: var(--color-critical); }
.severity-badge.severity-high { background-color: var(--color-high); }
.severity-badge.severity-medium { background-color: var(--color-medium); }
.severity-badge.severity-low { background-color: var(--color-low); }
.severity-badge.severity-info { background-color: var(--color-info); }

/* Violations */
.violation-card {
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 0.5rem;
  margin-bottom: 1.5rem;
  overflow: hidden;
}

.violation-card.severity-critical { border-left: 4px solid var(--color-critical); }
.violation-card.severity-high { border-left: 4px solid var(--color-high); }
.violation-card.severity-medium { border-left: 4px solid var(--color-medium); }
.violation-card.severity-low { border-left: 4px solid var(--color-low); }
.violation-card.severity-info { border-left: 4px solid var(--color-info); }

.violation-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  background-color: rgba(0, 0, 0, 0.02);
  border-bottom: 1px solid var(--color-border);
}

.violation-category {
  font-weight: 600;
}

.violation-id {
  margin-left: auto;
  font-size: 0.875rem;
  color: var(--color-text-secondary);
}

.violation-body {
  padding: 1rem;
}

.violation-description {
  font-size: 1.1rem;
}

.minimal-history, .root-cause-analysis, .evidence {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-border);
}

.operation-list {
  list-style: none;
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.operation-list li code {
  display: inline-block;
}

/* Timeline */
.timeline-section {
  margin-top: 2rem;
}

.timeline-container {
  overflow-x: auto;
  padding: 1rem 0;
}

.timeline {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 600px;
}

.timeline-event {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  background-color: var(--color-bg-secondary);
  border-radius: 0.25rem;
  border-left: 3px solid var(--color-info);
}

.timeline-event.operation-start { border-left-color: #3b82f6; }
.timeline-event.operation-end { border-left-color: #22c55e; }
.timeline-event.fault-injected { border-left-color: var(--color-high); }
.timeline-event.fault-healed { border-left-color: var(--color-pass); }
.timeline-event.violation-detected { border-left-color: var(--color-critical); }
.timeline-event.check-started { border-left-color: #8b5cf6; }
.timeline-event.check-completed { border-left-color: #8b5cf6; }
.timeline-event.phase-transition { border-left-color: #64748b; }

.timeline-time {
  font-family: monospace;
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  white-space: nowrap;
}

.timeline-label {
  font-weight: 500;
}

.timeline-description {
  color: var(--color-text-secondary);
  font-size: 0.875rem;
}

/* Coverage */
.coverage-percentage {
  font-size: 2.5rem;
  font-weight: bold;
  color: var(--color-pass);
}

.coverage-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 1rem;
}

.coverage-table th, .coverage-table td {
  padding: 0.5rem;
  text-align: left;
  border-bottom: 1px solid var(--color-border);
}

.coverage-table th {
  font-weight: 600;
  background-color: rgba(0, 0, 0, 0.02);
}

/* Recommendations */
.recommendation-card {
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 0.5rem;
  padding: 1.5rem;
  margin-bottom: 1rem;
}

.recommendation-card.priority-1 { border-left: 4px solid var(--color-critical); }
.recommendation-card.priority-2 { border-left: 4px solid var(--color-high); }
.recommendation-card.priority-3 { border-left: 4px solid var(--color-medium); }

.recommendation-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}

.recommendation-header .priority {
  background-color: var(--color-text);
  color: white;
  padding: 0.25rem 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  font-weight: bold;
}

/* Repro Bundle */
.repro-bundle-section .code-block {
  margin-bottom: 1.5rem;
}

.repro-instructions {
  background-color: #fef3c7;
  border: 1px solid #fcd34d;
  padding: 1rem;
  border-radius: 0.5rem;
  margin-bottom: 1.5rem;
}

/* Footer */
.report-footer {
  margin-top: 3rem;
  padding-top: 1.5rem;
  border-top: 2px solid var(--color-border);
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 0.875rem;
}

/* Utility */
.no-data, .no-violations {
  color: var(--color-text-secondary);
  font-style: italic;
  padding: 2rem;
  text-align: center;
  background-color: var(--color-bg-secondary);
  border-radius: 0.5rem;
}

/* Print styles */
@media print {
  body {
    max-width: none;
    padding: 1cm;
  }

  .timeline-container {
    overflow: visible;
  }

  .violation-card, .recommendation-card {
    break-inside: avoid;
  }
}
"#;

/// Embedded JavaScript for interactivity.
const EMBEDDED_JS: &str = r#"
document.addEventListener('DOMContentLoaded', function() {
  // Add click handlers for expandable sections
  document.querySelectorAll('.violation-card').forEach(function(card) {
    card.addEventListener('click', function(e) {
      if (e.target.tagName !== 'A' && e.target.tagName !== 'CODE') {
        this.classList.toggle('expanded');
      }
    });
  });

  // Timeline hover effects
  document.querySelectorAll('.timeline-event').forEach(function(event) {
    event.addEventListener('mouseenter', function() {
      this.style.backgroundColor = '#e5e7eb';
    });
    event.addEventListener('mouseleave', function() {
      this.style.backgroundColor = '';
    });
  });

  // Copy code blocks
  document.querySelectorAll('pre code').forEach(function(block) {
    const button = document.createElement('button');
    button.textContent = 'Copy';
    button.className = 'copy-button';
    button.style.cssText = 'position:absolute;top:0.5rem;right:0.5rem;padding:0.25rem 0.5rem;border:1px solid #ccc;border-radius:0.25rem;background:#fff;cursor:pointer;font-size:0.75rem;';

    const pre = block.parentElement;
    pre.style.position = 'relative';
    pre.appendChild(button);

    button.addEventListener('click', function() {
      navigator.clipboard.writeText(block.textContent).then(function() {
        button.textContent = 'Copied!';
        setTimeout(function() { button.textContent = 'Copy'; }, 2000);
      });
    });
  });
});
"#;

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
    fn test_html_generation() {
        let report = create_test_report();
        let generator = HtmlReportGenerator::new();

        let html = generator.generate(&report).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test Report"));
        assert!(html.contains("test-run-1"));
        assert!(html.contains("PASSED"));
    }

    #[test]
    fn test_html_escaping() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(&Duration::from_secs(30)), "30.000s");
        assert_eq!(format_duration(&Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(&Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_html_without_css() {
        let report = create_test_report();
        let config = HtmlReportConfig {
            embed_css: false,
            ..Default::default()
        };
        let generator = HtmlReportGenerator::with_config(config);

        let html = generator.generate(&report).unwrap();

        assert!(!html.contains(":root {"));
    }
}
