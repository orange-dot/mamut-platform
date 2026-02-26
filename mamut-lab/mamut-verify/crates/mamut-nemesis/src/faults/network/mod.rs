//! Network fault implementations.
//!
//! This module provides faults that affect network communication:
//!
//! - `NetworkPartition`: Blocks traffic between nodes using iptables
//! - `NetworkLatency`: Adds artificial latency using tc/netem

mod latency;
mod partition;

pub use latency::NetworkLatency;
pub use partition::NetworkPartition;

use std::net::IpAddr;

/// Helper for generating iptables commands.
pub(crate) struct IptablesCommand {
    /// The iptables binary (iptables or ip6tables).
    binary: &'static str,
    /// Chain to operate on.
    chain: String,
    /// Action (INPUT, OUTPUT, FORWARD).
    action: String,
}

impl IptablesCommand {
    /// Creates a new iptables command builder.
    pub fn new(is_ipv6: bool) -> Self {
        Self {
            binary: if is_ipv6 { "ip6tables" } else { "iptables" },
            chain: "FILTER".to_string(),
            action: "-A".to_string(),
        }
    }

    /// Sets the chain.
    pub fn chain(mut self, chain: &str) -> Self {
        self.chain = chain.to_string();
        self
    }

    /// Sets to insert mode.
    pub fn insert(mut self) -> Self {
        self.action = "-I".to_string();
        self
    }

    /// Sets to delete mode.
    pub fn delete(mut self) -> Self {
        self.action = "-D".to_string();
        self
    }

    /// Generates a DROP rule for traffic to a destination.
    pub fn drop_to(&self, dest: IpAddr, port: Option<u16>, protocol: Option<&str>) -> String {
        let mut cmd = format!(
            "{} {} OUTPUT -d {}",
            self.binary, self.action, dest
        );

        if let Some(proto) = protocol {
            cmd.push_str(&format!(" -p {}", proto));
        }

        if let Some(p) = port {
            cmd.push_str(&format!(" --dport {}", p));
        }

        cmd.push_str(" -j DROP");
        cmd
    }

    /// Generates a DROP rule for traffic from a source.
    pub fn drop_from(&self, source: IpAddr, port: Option<u16>, protocol: Option<&str>) -> String {
        let mut cmd = format!(
            "{} {} INPUT -s {}",
            self.binary, self.action, source
        );

        if let Some(proto) = protocol {
            cmd.push_str(&format!(" -p {}", proto));
        }

        if let Some(p) = port {
            cmd.push_str(&format!(" --sport {}", p));
        }

        cmd.push_str(" -j DROP");
        cmd
    }

    /// Generates a REJECT rule.
    pub fn reject_to(&self, dest: IpAddr, reject_with: &str) -> String {
        format!(
            "{} {} OUTPUT -d {} -j REJECT --reject-with {}",
            self.binary, self.action, dest, reject_with
        )
    }
}

/// Helper for generating tc (traffic control) commands.
pub(crate) struct TcCommand {
    /// Network interface.
    interface: String,
}

impl TcCommand {
    /// Creates a new tc command builder.
    pub fn new(interface: &str) -> Self {
        Self {
            interface: interface.to_string(),
        }
    }

    /// Adds a qdisc (queueing discipline).
    pub fn add_qdisc(&self, handle: &str, parent: &str, qdisc_type: &str) -> String {
        format!(
            "tc qdisc add dev {} parent {} handle {} {}",
            self.interface, parent, handle, qdisc_type
        )
    }

    /// Adds the root netem qdisc with delay.
    pub fn add_netem_delay(&self, delay_ms: u32, jitter_ms: Option<u32>) -> String {
        let mut cmd = format!(
            "tc qdisc add dev {} root netem delay {}ms",
            self.interface, delay_ms
        );

        if let Some(jitter) = jitter_ms {
            cmd.push_str(&format!(" {}ms", jitter));
        }

        cmd
    }

    /// Adds netem with delay and correlation.
    pub fn add_netem_delay_correlated(
        &self,
        delay_ms: u32,
        jitter_ms: u32,
        correlation_percent: u32,
    ) -> String {
        format!(
            "tc qdisc add dev {} root netem delay {}ms {}ms {}%",
            self.interface, delay_ms, jitter_ms, correlation_percent
        )
    }

    /// Adds netem with packet loss.
    pub fn add_netem_loss(&self, loss_percent: f32) -> String {
        format!(
            "tc qdisc add dev {} root netem loss {:.2}%",
            self.interface, loss_percent
        )
    }

    /// Adds netem with packet duplication.
    pub fn add_netem_duplicate(&self, duplicate_percent: f32) -> String {
        format!(
            "tc qdisc add dev {} root netem duplicate {:.2}%",
            self.interface, duplicate_percent
        )
    }

    /// Adds netem with packet corruption.
    pub fn add_netem_corrupt(&self, corrupt_percent: f32) -> String {
        format!(
            "tc qdisc add dev {} root netem corrupt {:.2}%",
            self.interface, corrupt_percent
        )
    }

    /// Adds netem with packet reordering.
    pub fn add_netem_reorder(&self, reorder_percent: f32, correlation_percent: u32) -> String {
        format!(
            "tc qdisc add dev {} root netem reorder {:.2}% {}%",
            self.interface, reorder_percent, correlation_percent
        )
    }

    /// Changes an existing qdisc.
    pub fn change_netem_delay(&self, delay_ms: u32) -> String {
        format!(
            "tc qdisc change dev {} root netem delay {}ms",
            self.interface, delay_ms
        )
    }

    /// Deletes the root qdisc.
    pub fn delete_root(&self) -> String {
        format!("tc qdisc del dev {} root", self.interface)
    }

    /// Shows current qdisc configuration.
    pub fn show(&self) -> String {
        format!("tc qdisc show dev {}", self.interface)
    }

    /// Adds a filter for specific traffic.
    pub fn add_filter(
        &self,
        parent: &str,
        protocol: &str,
        dest: Option<IpAddr>,
        dest_port: Option<u16>,
        flowid: &str,
    ) -> String {
        let mut cmd = format!(
            "tc filter add dev {} parent {} protocol {} prio 1 u32",
            self.interface, parent, protocol
        );

        if let Some(d) = dest {
            cmd.push_str(&format!(" match ip dst {}", d));
        }

        if let Some(p) = dest_port {
            cmd.push_str(&format!(" match ip dport {} 0xffff", p));
        }

        cmd.push_str(&format!(" flowid {}", flowid));
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_iptables_drop_command() {
        let cmd = IptablesCommand::new(false);
        let rule = cmd.drop_to(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            Some(8080),
            Some("tcp"),
        );

        assert!(rule.contains("iptables"));
        assert!(rule.contains("-d 192.168.1.1"));
        assert!(rule.contains("-p tcp"));
        assert!(rule.contains("--dport 8080"));
        assert!(rule.contains("-j DROP"));
    }

    #[test]
    fn test_tc_netem_delay() {
        let cmd = TcCommand::new("eth0");
        let rule = cmd.add_netem_delay(100, Some(20));

        assert!(rule.contains("tc qdisc add"));
        assert!(rule.contains("dev eth0"));
        assert!(rule.contains("netem delay 100ms"));
        assert!(rule.contains("20ms"));
    }

    #[test]
    fn test_tc_delete() {
        let cmd = TcCommand::new("eth0");
        let rule = cmd.delete_root();

        assert_eq!(rule, "tc qdisc del dev eth0 root");
    }
}
