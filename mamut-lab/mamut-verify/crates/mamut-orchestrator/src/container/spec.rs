//! Container specification types.
//!
//! This module provides types for container images and resource limits.

use serde::{Deserialize, Serialize};

/// Container image specification.
///
/// Represents a Docker image with optional registry, tag, and digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerImage {
    /// Full image reference (registry/repository:tag@digest).
    reference: String,

    /// Image pull policy.
    pub pull_policy: ImagePullPolicy,
}

impl ContainerImage {
    /// Creates a new container image from a reference string.
    ///
    /// # Examples
    ///
    /// ```
    /// use mamut_orchestrator::container::ContainerImage;
    ///
    /// let image = ContainerImage::new("redis:7-alpine");
    /// assert_eq!(image.reference(), "redis:7-alpine");
    /// ```
    pub fn new(reference: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            pull_policy: ImagePullPolicy::default(),
        }
    }

    /// Creates an image with a specific pull policy.
    pub fn with_pull_policy(mut self, policy: ImagePullPolicy) -> Self {
        self.pull_policy = policy;
        self
    }

    /// Returns the full image reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the image name (without tag or digest).
    pub fn name(&self) -> &str {
        self.reference
            .split('@')
            .next()
            .unwrap_or(&self.reference)
            .split(':')
            .next()
            .unwrap_or(&self.reference)
    }

    /// Returns the image tag if present.
    pub fn tag(&self) -> Option<&str> {
        let without_digest = self.reference.split('@').next()?;
        let parts: Vec<&str> = without_digest.split(':').collect();
        if parts.len() > 1 {
            Some(parts[1])
        } else {
            None
        }
    }

    /// Returns the image digest if present.
    pub fn digest(&self) -> Option<&str> {
        let parts: Vec<&str> = self.reference.split('@').collect();
        if parts.len() > 1 {
            Some(parts[1])
        } else {
            None
        }
    }

    /// Returns the registry if specified.
    pub fn registry(&self) -> Option<&str> {
        let name = self.name();
        if name.contains('/') {
            let parts: Vec<&str> = name.split('/').collect();
            // Check if first part looks like a registry (contains . or :)
            if parts[0].contains('.') || parts[0].contains(':') {
                return Some(parts[0]);
            }
        }
        None
    }
}

impl From<&str> for ContainerImage {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ContainerImage {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Image pull policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePullPolicy {
    /// Always pull the image.
    Always,

    /// Pull if not present locally.
    IfNotPresent,

    /// Never pull (image must be present).
    Never,
}

impl Default for ImagePullPolicy {
    fn default() -> Self {
        Self::IfNotPresent
    }
}

/// Resource limits for a container.
///
/// Defines CPU, memory, and other resource constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limit in millicores (1000 = 1 CPU).
    pub cpu_millicores: Option<u32>,

    /// Memory limit in bytes.
    pub memory_bytes: Option<u64>,

    /// Memory reservation (soft limit) in bytes.
    pub memory_reservation_bytes: Option<u64>,

    /// Swap limit in bytes (-1 for unlimited).
    pub swap_bytes: Option<i64>,

    /// CPU shares (relative weight).
    pub cpu_shares: Option<u32>,

    /// CPU quota in microseconds per period.
    pub cpu_quota: Option<i64>,

    /// CPU period in microseconds.
    pub cpu_period: Option<u64>,

    /// CPUs to use (e.g., "0-2" or "0,1").
    pub cpuset: Option<String>,

    /// Block I/O weight (10-1000).
    pub blkio_weight: Option<u16>,

    /// Process limit (pids).
    pub pids_limit: Option<i64>,

    /// Ulimits.
    pub ulimits: Vec<Ulimit>,

    /// OOM killer disable.
    pub oom_kill_disable: bool,

    /// OOM score adjustment (-1000 to 1000).
    pub oom_score_adj: Option<i32>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_millicores: None,
            memory_bytes: None,
            memory_reservation_bytes: None,
            swap_bytes: None,
            cpu_shares: None,
            cpu_quota: None,
            cpu_period: None,
            cpuset: None,
            blkio_weight: None,
            pids_limit: None,
            ulimits: Vec::new(),
            oom_kill_disable: false,
            oom_score_adj: None,
        }
    }
}

impl ResourceLimits {
    /// Creates a new resource limits builder.
    pub fn builder() -> ResourceLimitsBuilder {
        ResourceLimitsBuilder::default()
    }

    /// Creates resource limits with CPU and memory limits.
    pub fn new(cpu_millicores: u32, memory_mb: u64) -> Self {
        Self {
            cpu_millicores: Some(cpu_millicores),
            memory_bytes: Some(memory_mb * 1024 * 1024),
            ..Default::default()
        }
    }

    /// Returns the CPU limit as a fraction (e.g., 0.5 for 500m).
    pub fn cpu_limit(&self) -> Option<f64> {
        self.cpu_millicores.map(|m| m as f64 / 1000.0)
    }

    /// Returns the memory limit in megabytes.
    pub fn memory_mb(&self) -> Option<u64> {
        self.memory_bytes.map(|b| b / (1024 * 1024))
    }

    /// Returns the NanoCPUs value for Docker API.
    pub fn nano_cpus(&self) -> Option<i64> {
        self.cpu_millicores.map(|m| (m as i64) * 1_000_000)
    }
}

/// Builder for `ResourceLimits`.
#[derive(Debug, Default)]
pub struct ResourceLimitsBuilder {
    limits: ResourceLimits,
}

impl ResourceLimitsBuilder {
    /// Sets the CPU limit in millicores.
    pub fn cpu_millicores(mut self, millicores: u32) -> Self {
        self.limits.cpu_millicores = Some(millicores);
        self
    }

    /// Sets the CPU limit as a fraction (e.g., 0.5 for half a CPU).
    pub fn cpu(mut self, cpus: f64) -> Self {
        self.limits.cpu_millicores = Some((cpus * 1000.0) as u32);
        self
    }

    /// Sets the memory limit in bytes.
    pub fn memory_bytes(mut self, bytes: u64) -> Self {
        self.limits.memory_bytes = Some(bytes);
        self
    }

    /// Sets the memory limit in megabytes.
    pub fn memory_mb(mut self, mb: u64) -> Self {
        self.limits.memory_bytes = Some(mb * 1024 * 1024);
        self
    }

    /// Sets the memory limit in gigabytes.
    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.limits.memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Sets the memory reservation.
    pub fn memory_reservation_mb(mut self, mb: u64) -> Self {
        self.limits.memory_reservation_bytes = Some(mb * 1024 * 1024);
        self
    }

    /// Sets the swap limit.
    pub fn swap_mb(mut self, mb: i64) -> Self {
        if mb < 0 {
            self.limits.swap_bytes = Some(-1);
        } else {
            self.limits.swap_bytes = Some(mb * 1024 * 1024);
        }
        self
    }

    /// Sets the CPU shares.
    pub fn cpu_shares(mut self, shares: u32) -> Self {
        self.limits.cpu_shares = Some(shares);
        self
    }

    /// Sets the CPU set.
    pub fn cpuset(mut self, cpuset: impl Into<String>) -> Self {
        self.limits.cpuset = Some(cpuset.into());
        self
    }

    /// Sets the block I/O weight.
    pub fn blkio_weight(mut self, weight: u16) -> Self {
        self.limits.blkio_weight = Some(weight.clamp(10, 1000));
        self
    }

    /// Sets the process limit.
    pub fn pids_limit(mut self, limit: i64) -> Self {
        self.limits.pids_limit = Some(limit);
        self
    }

    /// Adds a ulimit.
    pub fn ulimit(mut self, ulimit: Ulimit) -> Self {
        self.limits.ulimits.push(ulimit);
        self
    }

    /// Disables the OOM killer.
    pub fn disable_oom_killer(mut self) -> Self {
        self.limits.oom_kill_disable = true;
        self
    }

    /// Sets the OOM score adjustment.
    pub fn oom_score_adj(mut self, adj: i32) -> Self {
        self.limits.oom_score_adj = Some(adj.clamp(-1000, 1000));
        self
    }

    /// Builds the resource limits.
    pub fn build(self) -> ResourceLimits {
        self.limits
    }
}

/// Ulimit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ulimit {
    /// Ulimit name (e.g., "nofile", "nproc").
    pub name: String,

    /// Soft limit.
    pub soft: i64,

    /// Hard limit.
    pub hard: i64,
}

impl Ulimit {
    /// Creates a new ulimit.
    pub fn new(name: impl Into<String>, soft: i64, hard: i64) -> Self {
        Self {
            name: name.into(),
            soft,
            hard,
        }
    }

    /// Creates a nofile ulimit.
    pub fn nofile(soft: i64, hard: i64) -> Self {
        Self::new("nofile", soft, hard)
    }

    /// Creates an nproc ulimit.
    pub fn nproc(soft: i64, hard: i64) -> Self {
        Self::new("nproc", soft, hard)
    }

    /// Creates a ulimit with same soft and hard limits.
    pub fn equal(name: impl Into<String>, limit: i64) -> Self {
        Self::new(name, limit, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_image_parsing() {
        let image = ContainerImage::new("redis:7-alpine");
        assert_eq!(image.name(), "redis");
        assert_eq!(image.tag(), Some("7-alpine"));
        assert_eq!(image.digest(), None);
        assert_eq!(image.registry(), None);
    }

    #[test]
    fn test_container_image_with_registry() {
        let image = ContainerImage::new("gcr.io/project/image:v1");
        assert_eq!(image.registry(), Some("gcr.io"));
        assert_eq!(image.name(), "gcr.io/project/image");
        assert_eq!(image.tag(), Some("v1"));
    }

    #[test]
    fn test_container_image_with_digest() {
        let image = ContainerImage::new("redis@sha256:abc123");
        assert_eq!(image.name(), "redis");
        assert_eq!(image.tag(), None);
        assert_eq!(image.digest(), Some("sha256:abc123"));
    }

    #[test]
    fn test_resource_limits_builder() {
        let limits = ResourceLimits::builder()
            .cpu(0.5)
            .memory_mb(512)
            .pids_limit(100)
            .build();

        assert_eq!(limits.cpu_millicores, Some(500));
        assert_eq!(limits.memory_bytes, Some(512 * 1024 * 1024));
        assert_eq!(limits.pids_limit, Some(100));
    }

    #[test]
    fn test_resource_limits_conversions() {
        let limits = ResourceLimits::new(500, 256);

        assert_eq!(limits.cpu_limit(), Some(0.5));
        assert_eq!(limits.memory_mb(), Some(256));
        assert_eq!(limits.nano_cpus(), Some(500_000_000));
    }

    #[test]
    fn test_ulimit() {
        let ulimit = Ulimit::nofile(65535, 65535);
        assert_eq!(ulimit.name, "nofile");
        assert_eq!(ulimit.soft, 65535);
        assert_eq!(ulimit.hard, 65535);
    }
}
