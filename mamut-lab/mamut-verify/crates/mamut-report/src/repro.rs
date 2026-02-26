//! ReproBundle generation for reproducibility.
//!
//! This module provides functionality to generate reproducibility bundles
//! that allow violations to be recreated in isolated environments.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use chrono::Utc;
use tracing::instrument;

use crate::types::{AuditReport, ReportError, ReproBundle, ViolationReport};

/// Configuration for ReproBundle generation.
#[derive(Debug, Clone)]
pub struct ReproBundleConfig {
    /// Docker image for the test runner.
    pub test_runner_image: String,
    /// Docker image for the system under test.
    pub system_image: Option<String>,
    /// Network name for Docker.
    pub network_name: String,
    /// Volume mounts.
    pub volumes: Vec<VolumeMount>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Include seed data in bundle.
    pub include_seed_data: bool,
    /// Include full history export.
    pub include_full_history: bool,
    /// Compression level for seed data (0-9).
    pub compression_level: u32,
}

impl Default for ReproBundleConfig {
    fn default() -> Self {
        Self {
            test_runner_image: "mamut/test-runner:latest".to_string(),
            system_image: None,
            network_name: "mamut-repro-network".to_string(),
            volumes: Vec::new(),
            environment: HashMap::new(),
            include_seed_data: true,
            include_full_history: false,
            compression_level: 6,
        }
    }
}

/// A volume mount specification.
#[derive(Debug, Clone)]
pub struct VolumeMount {
    /// Source path or volume name.
    pub source: String,
    /// Target path in container.
    pub target: String,
    /// Read-only mount.
    pub read_only: bool,
}

/// ReproBundle generator.
#[derive(Debug, Clone)]
pub struct ReproBundleGenerator {
    config: ReproBundleConfig,
}

impl ReproBundleGenerator {
    /// Create a new ReproBundle generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: ReproBundleConfig::default(),
        }
    }

    /// Create a ReproBundle generator with custom configuration.
    pub fn with_config(config: ReproBundleConfig) -> Self {
        Self { config }
    }

    /// Generate a ReproBundle from an audit report.
    #[instrument(skip(self, report), fields(report_id = %report.summary.report_id))]
    pub fn generate(&self, report: &AuditReport) -> Result<ReproBundle, ReportError> {
        let mut bundle = ReproBundle::new();

        // Generate Docker Compose
        bundle.docker_compose = self.generate_docker_compose(report)?;

        // Generate test script
        bundle.test_script = self.generate_test_script(report)?;

        // Generate expected violation description
        bundle.expected_violation = self.generate_expected_violation(report)?;

        // Export minimal history as seed data
        if self.config.include_seed_data {
            bundle.seed_data = self.export_seed_data(report)?;
        }

        // Set environment variables
        bundle.environment = self.config.environment.clone();
        bundle.environment.insert(
            "MAMUT_TEST_RUN_ID".to_string(),
            report.summary.test_run_id.clone(),
        );
        if let Some(seed) = report.test_configuration.seed {
            bundle
                .environment
                .insert("MAMUT_SEED".to_string(), seed.to_string());
        }

        // Generate instructions
        bundle.instructions = self.generate_instructions(report)?;

        // Add additional files
        bundle.additional_files = self.generate_additional_files(report)?;

        bundle.created_at = Utc::now();

        Ok(bundle)
    }

    /// Generate a ReproBundle for a specific violation.
    pub fn generate_for_violation(
        &self,
        report: &AuditReport,
        violation: &ViolationReport,
    ) -> Result<ReproBundle, ReportError> {
        let mut bundle = ReproBundle::new();

        bundle.docker_compose = self.generate_docker_compose(report)?;
        bundle.test_script = self.generate_violation_test_script(report, violation)?;
        bundle.expected_violation = format!(
            "{} violation ({}): {}",
            violation.severity, violation.category, violation.description
        );

        // Export minimal history for this violation
        bundle.seed_data = self.export_minimal_history(violation)?;

        bundle.environment = self.config.environment.clone();
        bundle.environment.insert(
            "MAMUT_TEST_RUN_ID".to_string(),
            report.summary.test_run_id.clone(),
        );
        bundle.environment.insert(
            "MAMUT_VIOLATION_ID".to_string(),
            violation.id.to_string(),
        );
        if let Some(seed) = report.test_configuration.seed {
            bundle
                .environment
                .insert("MAMUT_SEED".to_string(), seed.to_string());
        }

        bundle.instructions = format!(
            "This bundle reproduces violation {} ({}).\n\n{}",
            violation.id,
            violation.category,
            self.generate_instructions(report)?
        );

        bundle.created_at = Utc::now();

        Ok(bundle)
    }

    fn generate_docker_compose(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut compose = String::new();

        compose.push_str("version: '3.8'\n\n");

        // Networks
        compose.push_str("networks:\n");
        compose.push_str(&format!("  {}:\n", self.config.network_name));
        compose.push_str("    driver: bridge\n\n");

        // Volumes
        compose.push_str("volumes:\n");
        compose.push_str("  mamut-data:\n");
        compose.push_str("  mamut-history:\n\n");

        // Services
        compose.push_str("services:\n");

        // System under test
        let system_image = self.config.system_image.as_ref().map_or_else(
            || format!("{}:latest", report.test_configuration.system_under_test),
            |s| s.clone(),
        );

        compose.push_str("  system-under-test:\n");
        compose.push_str(&format!("    image: {}\n", system_image));
        compose.push_str(&format!(
            "    container_name: mamut-sut-{}\n",
            sanitize_container_name(&report.summary.test_run_id)
        ));
        compose.push_str("    networks:\n");
        compose.push_str(&format!("      - {}\n", self.config.network_name));
        compose.push_str("    volumes:\n");
        compose.push_str("      - mamut-data:/data\n");
        compose.push_str("    healthcheck:\n");
        compose.push_str("      test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:8080/health\"]\n");
        compose.push_str("      interval: 5s\n");
        compose.push_str("      timeout: 3s\n");
        compose.push_str("      retries: 5\n");
        compose.push_str("    restart: unless-stopped\n\n");

        // Test runner
        compose.push_str("  test-runner:\n");
        compose.push_str(&format!("    image: {}\n", self.config.test_runner_image));
        compose.push_str(&format!(
            "    container_name: mamut-runner-{}\n",
            sanitize_container_name(&report.summary.test_run_id)
        ));
        compose.push_str("    networks:\n");
        compose.push_str(&format!("      - {}\n", self.config.network_name));
        compose.push_str("    volumes:\n");
        compose.push_str("      - mamut-history:/history\n");
        compose.push_str("      - ./seed-data:/seed-data:ro\n");
        compose.push_str("      - ./test-script.sh:/test-script.sh:ro\n");
        compose.push_str("    environment:\n");
        compose.push_str("      - MAMUT_SUT_HOST=system-under-test\n");
        compose.push_str(&format!(
            "      - MAMUT_TEST_NAME={}\n",
            report.test_configuration.name
        ));
        compose.push_str(&format!(
            "      - MAMUT_CONSISTENCY_MODEL={}\n",
            report.test_configuration.consistency_model
        ));
        compose.push_str(&format!(
            "      - MAMUT_CONCURRENCY={}\n",
            report.test_configuration.concurrency
        ));
        if let Some(seed) = report.test_configuration.seed {
            compose.push_str(&format!("      - MAMUT_SEED={}\n", seed));
        }

        // Add custom environment variables
        for (key, value) in &self.config.environment {
            compose.push_str(&format!("      - {}={}\n", key, value));
        }

        compose.push_str("    depends_on:\n");
        compose.push_str("      system-under-test:\n");
        compose.push_str("        condition: service_healthy\n");
        compose.push_str("    command: [\"/bin/bash\", \"/test-script.sh\"]\n");

        Ok(compose)
    }

    fn generate_test_script(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut script = String::new();

        script.push_str("#!/bin/bash\n");
        script.push_str("set -euo pipefail\n\n");

        script.push_str("# Mamut Verification Test Script\n");
        script.push_str(&format!(
            "# Generated for test run: {}\n",
            report.summary.test_run_id
        ));
        script.push_str(&format!("# Test: {}\n", report.test_configuration.name));
        script.push_str(&format!(
            "# Consistency Model: {}\n\n",
            report.test_configuration.consistency_model
        ));

        script.push_str("echo \"Starting Mamut Verification Test\"\n");
        script.push_str("echo \"================================\"\n\n");

        // Wait for system to be ready
        script.push_str("# Wait for system under test\n");
        script.push_str("echo \"Waiting for system under test to be ready...\"\n");
        script.push_str("max_retries=30\n");
        script.push_str("retry_count=0\n");
        script.push_str("until curl -sf http://${MAMUT_SUT_HOST}:8080/health > /dev/null 2>&1; do\n");
        script.push_str("    retry_count=$((retry_count + 1))\n");
        script.push_str("    if [ $retry_count -ge $max_retries ]; then\n");
        script.push_str("        echo \"ERROR: System under test did not become ready\"\n");
        script.push_str("        exit 1\n");
        script.push_str("    fi\n");
        script.push_str("    echo \"Retry $retry_count/$max_retries...\"\n");
        script.push_str("    sleep 2\n");
        script.push_str("done\n");
        script.push_str("echo \"System under test is ready\"\n\n");

        // Load seed data if available
        script.push_str("# Load seed data\n");
        script.push_str("if [ -f /seed-data/history.json ]; then\n");
        script.push_str("    echo \"Loading seed data...\"\n");
        script.push_str("    mamut-loader --input /seed-data/history.json --target http://${MAMUT_SUT_HOST}:8080\n");
        script.push_str("fi\n\n");

        // Run the test
        script.push_str("# Run verification test\n");
        script.push_str("echo \"Running verification test...\"\n");
        script.push_str(&format!(
            "mamut-verify \\\n    --model {} \\\n",
            report.test_configuration.consistency_model
        ));
        script.push_str(&format!(
            "    --concurrency {} \\\n",
            report.test_configuration.concurrency
        ));
        script.push_str(&format!(
            "    --timeout {}s \\\n",
            report.test_configuration.timeout.as_secs()
        ));
        if let Some(seed) = report.test_configuration.seed {
            script.push_str(&format!("    --seed {} \\\n", seed));
        }
        script.push_str("    --target http://${MAMUT_SUT_HOST}:8080 \\\n");
        script.push_str("    --output /history/results.json\n\n");

        // Report results
        script.push_str("# Report results\n");
        script.push_str("echo \"\"\n");
        script.push_str("echo \"Test Results\"\n");
        script.push_str("echo \"============\"\n");
        script.push_str("if [ -f /history/results.json ]; then\n");
        script.push_str("    cat /history/results.json | jq '.summary'\n");
        script.push_str("fi\n");

        Ok(script)
    }

    fn generate_violation_test_script(
        &self,
        report: &AuditReport,
        violation: &ViolationReport,
    ) -> Result<String, ReportError> {
        let mut script = self.generate_test_script(report)?;

        // Add violation-specific replay
        script.push_str("\n# Replay minimal history for specific violation\n");
        script.push_str(&format!(
            "echo \"Replaying violation {} ({})\"\n",
            violation.id, violation.category
        ));
        script.push_str("if [ -f /seed-data/minimal-history.json ]; then\n");
        script.push_str(
            "    mamut-replay --input /seed-data/minimal-history.json --target http://${MAMUT_SUT_HOST}:8080\n",
        );
        script.push_str("fi\n");

        Ok(script)
    }

    fn generate_expected_violation(&self, report: &AuditReport) -> Result<String, ReportError> {
        if report.violations.is_empty() {
            return Ok("No violations expected (this bundle is for a passing test).".to_string());
        }

        let mut desc = String::new();
        desc.push_str(&format!(
            "Expected {} violation(s):\n\n",
            report.violations.len()
        ));

        for (i, violation) in report.violations.iter().enumerate() {
            desc.push_str(&format!(
                "{}. {} {} - {}\n",
                i + 1,
                violation.severity,
                violation.category,
                violation.description
            ));
            desc.push_str(&format!(
                "   Minimal history: {} operations (reduced from {})\n\n",
                violation.minimal_history.minimized_size, violation.minimal_history.original_size
            ));
        }

        Ok(desc)
    }

    fn export_seed_data(&self, report: &AuditReport) -> Result<Vec<u8>, ReportError> {
        // Export history analysis and violation data as JSON
        let seed_data = SeedData {
            test_run_id: report.summary.test_run_id.clone(),
            test_configuration: SeedTestConfig {
                name: report.test_configuration.name.clone(),
                consistency_model: report.test_configuration.consistency_model.clone(),
                concurrency: report.test_configuration.concurrency,
                seed: report.test_configuration.seed,
            },
            history_analysis: SeedHistoryAnalysis {
                total_operations: report.history_analysis.total_operations,
                operations_by_type: report.history_analysis.operations_by_type.clone(),
            },
            violations: report
                .violations
                .iter()
                .map(|v| SeedViolation {
                    id: v.id.to_string(),
                    category: v.category.to_string(),
                    operation_ids: v.minimal_history.operation_ids.clone(),
                })
                .collect(),
        };

        let json = serde_json::to_vec_pretty(&seed_data)?;

        Ok(json)
    }

    fn export_minimal_history(&self, violation: &ViolationReport) -> Result<Vec<u8>, ReportError> {
        let minimal_data = MinimalHistoryExport {
            violation_id: violation.id.to_string(),
            category: violation.category.to_string(),
            description: violation.description.clone(),
            operation_ids: violation.minimal_history.operation_ids.clone(),
            original_size: violation.minimal_history.original_size,
            minimized_size: violation.minimal_history.minimized_size,
        };

        let json = serde_json::to_vec_pretty(&minimal_data)?;

        Ok(json)
    }

    fn generate_instructions(&self, report: &AuditReport) -> Result<String, ReportError> {
        let mut instructions = String::new();

        instructions.push_str("# Reproduction Instructions\n\n");
        instructions.push_str("## Prerequisites\n\n");
        instructions.push_str("- Docker and Docker Compose installed\n");
        instructions.push_str("- At least 4GB of available RAM\n");
        instructions.push_str("- Network access to pull container images\n\n");

        instructions.push_str("## Quick Start\n\n");
        instructions.push_str("1. Extract all files to a directory\n");
        instructions.push_str("2. Run: `docker-compose up -d`\n");
        instructions.push_str("3. Wait for the test to complete\n");
        instructions.push_str("4. Check results: `docker-compose logs test-runner`\n\n");

        instructions.push_str("## Manual Reproduction\n\n");
        instructions.push_str("1. Start the system under test:\n");
        instructions.push_str("   ```bash\n");
        instructions.push_str("   docker-compose up -d system-under-test\n");
        instructions.push_str("   ```\n\n");
        instructions.push_str("2. Wait for it to be healthy:\n");
        instructions.push_str("   ```bash\n");
        instructions
            .push_str("   docker-compose exec system-under-test curl -f http://localhost:8080/health\n");
        instructions.push_str("   ```\n\n");
        instructions.push_str("3. Run the test:\n");
        instructions.push_str("   ```bash\n");
        instructions.push_str("   docker-compose run test-runner\n");
        instructions.push_str("   ```\n\n");

        instructions.push_str("## Test Configuration\n\n");
        instructions.push_str(&format!(
            "- Test Name: {}\n",
            report.test_configuration.name
        ));
        instructions.push_str(&format!(
            "- Consistency Model: {}\n",
            report.test_configuration.consistency_model
        ));
        instructions.push_str(&format!(
            "- Concurrency: {} processes\n",
            report.test_configuration.concurrency
        ));
        if let Some(seed) = report.test_configuration.seed {
            instructions.push_str(&format!("- Random Seed: {}\n", seed));
        }
        instructions.push_str(&format!(
            "- Timeout: {}s\n",
            report.test_configuration.timeout.as_secs()
        ));

        Ok(instructions)
    }

    fn generate_additional_files(
        &self,
        _report: &AuditReport,
    ) -> Result<HashMap<String, String>, ReportError> {
        let mut files = HashMap::new();

        // Add a README
        files.insert("README.md".to_string(), self.generate_readme()?);

        // Add a .env template
        files.insert(".env.template".to_string(), self.generate_env_template()?);

        Ok(files)
    }

    fn generate_readme(&self) -> Result<String, ReportError> {
        Ok(r#"# Mamut Verification Reproduction Bundle

This bundle contains everything needed to reproduce a verification test result.

## Contents

- `docker-compose.yml` - Docker Compose configuration
- `test-script.sh` - Test execution script
- `seed-data/` - Initial data for the test
- `.env.template` - Environment variable template

## Usage

See the instructions in the main bundle for reproduction steps.

## Support

For issues with this bundle, please contact the Mamut team.
"#
        .to_string())
    }

    fn generate_env_template(&self) -> Result<String, ReportError> {
        Ok(r#"# Mamut Test Environment Variables

# System Under Test
MAMUT_SUT_HOST=system-under-test
MAMUT_SUT_PORT=8080

# Test Configuration
# MAMUT_SEED=           # Override random seed
# MAMUT_CONCURRENCY=    # Override concurrency level
# MAMUT_TIMEOUT=        # Override timeout (seconds)

# Logging
MAMUT_LOG_LEVEL=info
MAMUT_LOG_FORMAT=json
"#
        .to_string())
    }

    /// Write the bundle to a directory.
    #[instrument(skip(self, bundle), fields(path = %path.as_ref().display()))]
    pub fn write_to_directory(
        &self,
        bundle: &ReproBundle,
        path: impl AsRef<Path>,
    ) -> Result<(), ReportError> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;

        // Write docker-compose.yml
        std::fs::write(path.join("docker-compose.yml"), &bundle.docker_compose)?;

        // Write test script
        let script_path = path.join("test-script.sh");
        std::fs::write(&script_path, &bundle.test_script)?;

        // Make script executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms)?;
        }

        // Write seed data
        let seed_dir = path.join("seed-data");
        std::fs::create_dir_all(&seed_dir)?;
        std::fs::write(seed_dir.join("history.json"), &bundle.seed_data)?;

        // Write .env file
        let mut env_content = String::new();
        for (key, value) in &bundle.environment {
            env_content.push_str(&format!("{}={}\n", key, value));
        }
        std::fs::write(path.join(".env"), &env_content)?;

        // Write instructions
        std::fs::write(path.join("INSTRUCTIONS.md"), &bundle.instructions)?;

        // Write additional files
        for (filename, content) in &bundle.additional_files {
            std::fs::write(path.join(filename), content)?;
        }

        Ok(())
    }

    /// Write the bundle to a tar.gz archive.
    pub fn write_to_archive<W: Write>(
        &self,
        bundle: &ReproBundle,
        writer: W,
    ) -> Result<(), ReportError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let compression = Compression::new(self.config.compression_level);
        let encoder = GzEncoder::new(writer, compression);
        let mut archive = tar::Builder::new(encoder);

        // Add docker-compose.yml
        add_file_to_archive(
            &mut archive,
            "docker-compose.yml",
            bundle.docker_compose.as_bytes(),
        )?;

        // Add test script
        add_file_to_archive(&mut archive, "test-script.sh", bundle.test_script.as_bytes())?;

        // Add seed data
        add_file_to_archive(&mut archive, "seed-data/history.json", &bundle.seed_data)?;

        // Add .env
        let mut env_content = String::new();
        for (key, value) in &bundle.environment {
            env_content.push_str(&format!("{}={}\n", key, value));
        }
        add_file_to_archive(&mut archive, ".env", env_content.as_bytes())?;

        // Add instructions
        add_file_to_archive(&mut archive, "INSTRUCTIONS.md", bundle.instructions.as_bytes())?;

        // Add additional files
        for (filename, content) in &bundle.additional_files {
            add_file_to_archive(&mut archive, filename, content.as_bytes())?;
        }

        archive.finish()?;

        Ok(())
    }
}

impl Default for ReproBundleGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Add a file to a tar archive.
fn add_file_to_archive<W: Write>(
    archive: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
) -> Result<(), ReportError> {
    let mut header = tar::Header::new_gnu();
    header.set_path(path).map_err(|e| {
        ReportError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    })?;
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );
    header.set_cksum();

    archive.append(&header, data)?;

    Ok(())
}

/// Sanitize a string for use in container names.
fn sanitize_container_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(63) // Docker container name limit
        .collect()
}

/// Seed data structure for JSON export.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SeedData {
    test_run_id: String,
    test_configuration: SeedTestConfig,
    history_analysis: SeedHistoryAnalysis,
    violations: Vec<SeedViolation>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SeedTestConfig {
    name: String,
    consistency_model: String,
    concurrency: u32,
    seed: Option<u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SeedHistoryAnalysis {
    total_operations: u64,
    operations_by_type: HashMap<String, u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SeedViolation {
    id: String,
    category: String,
    operation_ids: Vec<String>,
}

/// Minimal history export structure.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MinimalHistoryExport {
    violation_id: String,
    category: String,
    description: String,
    operation_ids: Vec<String>,
    original_size: usize,
    minimized_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        MinimalHistoryRef, ReportSummary, Severity, TestConfiguration, ViolationCategory,
    };
    use std::time::Duration;

    fn create_test_report() -> AuditReport {
        let summary = ReportSummary::new("test-run-1".to_string(), "Test Report".to_string());
        let config = TestConfiguration {
            name: "test".to_string(),
            description: None,
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
    fn test_repro_bundle_generation() {
        let report = create_test_report();
        let generator = ReproBundleGenerator::new();

        let bundle = generator.generate(&report).unwrap();

        assert!(bundle.docker_compose.contains("version:"));
        assert!(bundle.docker_compose.contains("system-under-test"));
        assert!(bundle.docker_compose.contains("test-runner"));
        assert!(bundle.test_script.contains("#!/bin/bash"));
        assert!(!bundle.instructions.is_empty());
    }

    #[test]
    fn test_docker_compose_structure() {
        let report = create_test_report();
        let generator = ReproBundleGenerator::new();

        let bundle = generator.generate(&report).unwrap();

        assert!(bundle.docker_compose.contains("networks:"));
        assert!(bundle.docker_compose.contains("volumes:"));
        assert!(bundle.docker_compose.contains("services:"));
        assert!(bundle.docker_compose.contains("healthcheck:"));
    }

    #[test]
    fn test_test_script_content() {
        let report = create_test_report();
        let generator = ReproBundleGenerator::new();

        let bundle = generator.generate(&report).unwrap();

        assert!(bundle.test_script.contains("set -euo pipefail"));
        assert!(bundle.test_script.contains("mamut-verify"));
        assert!(bundle.test_script.contains("--seed 42"));
    }

    #[test]
    fn test_violation_specific_bundle() {
        let mut report = create_test_report();
        let violation = ViolationReport::new(
            Severity::High,
            ViolationCategory::Linearizability,
            "Test violation".to_string(),
            MinimalHistoryRef {
                id: "h1".to_string(),
                operation_ids: vec!["op1".to_string()],
                original_size: 100,
                minimized_size: 1,
                description: "Test".to_string(),
            },
        );
        report.violations.push(violation.clone());

        let generator = ReproBundleGenerator::new();
        let bundle = generator
            .generate_for_violation(&report, &violation)
            .unwrap();

        assert!(bundle.expected_violation.contains("Linearizability"));
        assert!(bundle
            .environment
            .contains_key("MAMUT_VIOLATION_ID"));
    }

    #[test]
    fn test_sanitize_container_name() {
        assert_eq!(sanitize_container_name("test-run-1"), "test-run-1");
        assert_eq!(sanitize_container_name("Test Run 1"), "test-run-1");
        assert_eq!(
            sanitize_container_name("test@run#1"),
            "test-run-1"
        );
    }
}
