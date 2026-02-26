//! Timeline visualization data generation.
//!
//! This module provides timeline data structures and generation for
//! visualizing test execution, fault injection, and violations over time.

use std::collections::HashMap;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{AuditReport, ViolationReport};

/// Timeline event type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineEventType {
    /// Operation started.
    OperationStart,
    /// Operation completed.
    OperationEnd,
    /// Fault was injected.
    FaultInjected,
    /// Fault was healed.
    FaultHealed,
    /// Violation was detected.
    ViolationDetected,
    /// Consistency check started.
    CheckStarted,
    /// Consistency check completed.
    CheckCompleted,
    /// Test phase transition.
    PhaseTransition,
    /// Custom event type.
    Custom(String),
}

impl TimelineEventType {
    /// Get the CSS class for this event type.
    pub fn css_class(&self) -> &'static str {
        match self {
            TimelineEventType::OperationStart => "operation-start",
            TimelineEventType::OperationEnd => "operation-end",
            TimelineEventType::FaultInjected => "fault-injected",
            TimelineEventType::FaultHealed => "fault-healed",
            TimelineEventType::ViolationDetected => "violation-detected",
            TimelineEventType::CheckStarted => "check-started",
            TimelineEventType::CheckCompleted => "check-completed",
            TimelineEventType::PhaseTransition => "phase-transition",
            TimelineEventType::Custom(_) => "custom-event",
        }
    }

    /// Get the display name for this event type.
    pub fn display_name(&self) -> String {
        match self {
            TimelineEventType::OperationStart => "Operation Started".to_string(),
            TimelineEventType::OperationEnd => "Operation Completed".to_string(),
            TimelineEventType::FaultInjected => "Fault Injected".to_string(),
            TimelineEventType::FaultHealed => "Fault Healed".to_string(),
            TimelineEventType::ViolationDetected => "Violation Detected".to_string(),
            TimelineEventType::CheckStarted => "Check Started".to_string(),
            TimelineEventType::CheckCompleted => "Check Completed".to_string(),
            TimelineEventType::PhaseTransition => "Phase Transition".to_string(),
            TimelineEventType::Custom(name) => name.clone(),
        }
    }
}

/// A single event on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Unique identifier for this event.
    pub id: String,
    /// Event type.
    pub event_type: TimelineEventType,
    /// Timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// Display label for the event.
    pub label: String,
    /// Optional description.
    pub description: Option<String>,
    /// Process or node associated with this event.
    pub process_id: Option<String>,
    /// Duration of the event (for operations).
    pub duration_ms: Option<i64>,
    /// Related event IDs.
    pub related_events: Vec<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TimelineEvent {
    /// Create a new timeline event.
    pub fn new(
        id: String,
        event_type: TimelineEventType,
        timestamp: DateTime<Utc>,
        label: String,
    ) -> Self {
        Self {
            id,
            event_type,
            timestamp,
            label,
            description: None,
            process_id: None,
            duration_ms: None,
            related_events: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the process ID.
    pub fn with_process_id(mut self, process_id: String) -> Self {
        self.process_id = Some(process_id);
        self
    }

    /// Set the duration.
    pub fn with_duration_ms(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Add a related event.
    pub fn with_related_event(mut self, event_id: String) -> Self {
        self.related_events.push(event_id);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// A span representing a time range on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSpan {
    /// Unique identifier for this span.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Span category.
    pub category: String,
    /// Start timestamp.
    pub start: DateTime<Utc>,
    /// End timestamp.
    pub end: DateTime<Utc>,
    /// CSS class for styling.
    pub css_class: String,
    /// Process or node associated with this span.
    pub process_id: Option<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TimelineSpan {
    /// Create a new timeline span.
    pub fn new(
        id: String,
        label: String,
        category: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            label,
            category: category.clone(),
            start,
            end,
            css_class: format!("span-{}", category.to_lowercase().replace(' ', "-")),
            process_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Get the duration of this span.
    pub fn duration(&self) -> ChronoDuration {
        self.end - self.start
    }
}

/// A lane (track) on the timeline, typically representing a process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLane {
    /// Lane identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Lane order (for sorting).
    pub order: i32,
    /// Events in this lane.
    pub events: Vec<TimelineEvent>,
    /// Spans in this lane.
    pub spans: Vec<TimelineSpan>,
}

impl TimelineLane {
    /// Create a new timeline lane.
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            order: 0,
            events: Vec::new(),
            spans: Vec::new(),
        }
    }

    /// Add an event to this lane.
    pub fn add_event(&mut self, event: TimelineEvent) {
        self.events.push(event);
    }

    /// Add a span to this lane.
    pub fn add_span(&mut self, span: TimelineSpan) {
        self.spans.push(span);
    }

    /// Sort events and spans by timestamp.
    pub fn sort(&mut self) {
        self.events.sort_by_key(|e| e.timestamp);
        self.spans.sort_by_key(|s| s.start);
    }
}

/// Complete timeline data for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineData {
    /// Timeline title.
    pub title: String,
    /// Timeline description.
    pub description: Option<String>,
    /// Start of the timeline.
    pub start_time: Option<DateTime<Utc>>,
    /// End of the timeline.
    pub end_time: Option<DateTime<Utc>>,
    /// All events (not organized by lane).
    pub events: Vec<TimelineEvent>,
    /// Lanes (tracks) on the timeline.
    pub lanes: Vec<TimelineLane>,
    /// Markers for important moments.
    pub markers: Vec<TimelineMarker>,
    /// Legend entries.
    pub legend: Vec<LegendEntry>,
}

impl TimelineData {
    /// Create a new empty timeline.
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            start_time: None,
            end_time: None,
            events: Vec::new(),
            lanes: Vec::new(),
            markers: Vec::new(),
            legend: Vec::new(),
        }
    }

    /// Add an event to the timeline.
    pub fn add_event(&mut self, event: TimelineEvent) {
        // Update time bounds
        if self.start_time.is_none() || event.timestamp < self.start_time.unwrap() {
            self.start_time = Some(event.timestamp);
        }
        if self.end_time.is_none() || event.timestamp > self.end_time.unwrap() {
            self.end_time = Some(event.timestamp);
        }
        self.events.push(event);
    }

    /// Add a lane to the timeline.
    pub fn add_lane(&mut self, lane: TimelineLane) {
        self.lanes.push(lane);
    }

    /// Add a marker to the timeline.
    pub fn add_marker(&mut self, marker: TimelineMarker) {
        self.markers.push(marker);
    }

    /// Sort all events and lanes by timestamp.
    pub fn sort(&mut self) {
        self.events.sort_by_key(|e| e.timestamp);
        for lane in &mut self.lanes {
            lane.sort();
        }
        self.markers.sort_by_key(|m| m.timestamp);
    }

    /// Get the duration of the timeline.
    pub fn duration(&self) -> Option<ChronoDuration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// Build default legend from event types.
    pub fn build_legend(&mut self) {
        let mut seen = std::collections::HashSet::new();

        for event in &self.events {
            let type_name = event.event_type.display_name();
            if seen.insert(type_name.clone()) {
                self.legend.push(LegendEntry {
                    label: type_name,
                    css_class: event.event_type.css_class().to_string(),
                    description: None,
                });
            }
        }
    }
}

/// A marker on the timeline (vertical line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    /// Marker identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// CSS class for styling.
    pub css_class: String,
    /// Optional description.
    pub description: Option<String>,
}

impl TimelineMarker {
    /// Create a new timeline marker.
    pub fn new(id: String, label: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            label,
            timestamp,
            css_class: "marker".to_string(),
            description: None,
        }
    }

    /// Set the CSS class.
    pub fn with_css_class(mut self, css_class: String) -> Self {
        self.css_class = css_class;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// Legend entry for the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegendEntry {
    /// Display label.
    pub label: String,
    /// CSS class for the color/style.
    pub css_class: String,
    /// Optional description.
    pub description: Option<String>,
}

/// Timeline generator for creating timeline data from reports.
#[derive(Debug, Clone, Default)]
pub struct TimelineGenerator {
    /// Include operation events.
    include_operations: bool,
    /// Include fault events.
    include_faults: bool,
    /// Include violation events.
    include_violations: bool,
    /// Maximum events to include.
    max_events: usize,
}

impl TimelineGenerator {
    /// Create a new timeline generator with default settings.
    pub fn new() -> Self {
        Self {
            include_operations: true,
            include_faults: true,
            include_violations: true,
            max_events: 1000,
        }
    }

    /// Set whether to include operation events.
    pub fn with_operations(mut self, include: bool) -> Self {
        self.include_operations = include;
        self
    }

    /// Set whether to include fault events.
    pub fn with_faults(mut self, include: bool) -> Self {
        self.include_faults = include;
        self
    }

    /// Set whether to include violation events.
    pub fn with_violations(mut self, include: bool) -> Self {
        self.include_violations = include;
        self
    }

    /// Set maximum events to include.
    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Generate timeline data from an audit report.
    pub fn generate_from_report(&self, report: &AuditReport) -> TimelineData {
        let mut timeline = TimelineData::new(format!("{} - Timeline", report.summary.title));
        timeline.description = Some(format!(
            "Execution timeline for test run {}",
            report.summary.test_run_id
        ));

        // Set time bounds from history analysis
        if let Some((start, end)) = report.history_analysis.time_span {
            timeline.start_time = Some(start);
            timeline.end_time = Some(end);
        }

        // Add violation events
        if self.include_violations {
            for violation in &report.violations {
                self.add_violation_event(&mut timeline, violation);
            }
        }

        // Add test start/end markers
        if let Some(start) = timeline.start_time {
            timeline.add_marker(
                TimelineMarker::new("test-start".to_string(), "Test Started".to_string(), start)
                    .with_css_class("marker-start".to_string()),
            );
        }

        if let Some(end) = timeline.end_time {
            timeline.add_marker(
                TimelineMarker::new("test-end".to_string(), "Test Ended".to_string(), end)
                    .with_css_class("marker-end".to_string()),
            );
        }

        // Sort and build legend
        timeline.sort();
        timeline.build_legend();

        timeline
    }

    fn add_violation_event(&self, timeline: &mut TimelineData, violation: &ViolationReport) {
        let event = TimelineEvent::new(
            format!("violation-{}", violation.id),
            TimelineEventType::ViolationDetected,
            violation.detected_at,
            format!("{} Violation", violation.category),
        )
        .with_description(violation.description.clone())
        .with_metadata(
            "severity".to_string(),
            serde_json::Value::String(violation.severity.to_string()),
        )
        .with_metadata(
            "category".to_string(),
            serde_json::Value::String(violation.category.to_string()),
        );

        timeline.add_event(event);
    }

    /// Generate timeline data from history events.
    pub fn generate_from_events(&self, events: &[TimelineEvent]) -> TimelineData {
        let mut timeline = TimelineData::new("Event Timeline".to_string());

        let events_to_add = if self.max_events > 0 && events.len() > self.max_events {
            &events[..self.max_events]
        } else {
            events
        };

        for event in events_to_add {
            let include = match event.event_type {
                TimelineEventType::OperationStart | TimelineEventType::OperationEnd => {
                    self.include_operations
                }
                TimelineEventType::FaultInjected | TimelineEventType::FaultHealed => {
                    self.include_faults
                }
                TimelineEventType::ViolationDetected => self.include_violations,
                _ => true,
            };

            if include {
                timeline.add_event(event.clone());
            }
        }

        timeline.sort();
        timeline.build_legend();

        timeline
    }
}

/// Convert timeline data to JSON for JavaScript visualization libraries.
pub fn timeline_to_json(timeline: &TimelineData) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(timeline)
}

/// Convert timeline data to a simple SVG representation.
pub fn timeline_to_svg(timeline: &TimelineData, width: u32, height: u32) -> String {
    let mut svg = String::new();

    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        width, height, width, height
    ));

    // Styles
    svg.push_str(r##"<style>
        .timeline-axis { stroke: #333; stroke-width: 2; }
        .event-marker { fill: #3b82f6; }
        .event-marker.violation { fill: #dc2626; }
        .event-marker.fault { fill: #ea580c; }
        .event-label { font-family: sans-serif; font-size: 10px; }
    </style>"##);

    // Background
    svg.push_str(&format!(
        r##"<rect width="{}" height="{}" fill="#f8fafc"/>"##,
        width, height
    ));

    // Title
    svg.push_str(&format!(
        r#"<text x="{}" y="20" text-anchor="middle" class="event-label" style="font-size: 14px; font-weight: bold;">{}</text>"#,
        width / 2,
        escape_svg(&timeline.title)
    ));

    // Draw timeline axis
    let margin = 60;
    let axis_y = height - margin;
    svg.push_str(&format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="timeline-axis"/>"#,
        margin,
        axis_y,
        width - margin,
        axis_y
    ));

    // Draw events
    if let (Some(start), Some(end)) = (timeline.start_time, timeline.end_time) {
        let duration_ms = (end - start).num_milliseconds() as f64;
        if duration_ms > 0.0 {
            let available_width = (width - 2 * margin) as f64;

            for (i, event) in timeline.events.iter().enumerate() {
                let offset_ms = (event.timestamp - start).num_milliseconds() as f64;
                let x = margin as f64 + (offset_ms / duration_ms) * available_width;
                let y = axis_y as f64 - 30.0 - ((i % 3) as f64 * 20.0);

                let class = match event.event_type {
                    TimelineEventType::ViolationDetected => "event-marker violation",
                    TimelineEventType::FaultInjected | TimelineEventType::FaultHealed => {
                        "event-marker fault"
                    }
                    _ => "event-marker",
                };

                svg.push_str(&format!(
                    r#"<circle cx="{:.1}" cy="{:.1}" r="5" class="{}"><title>{}</title></circle>"#,
                    x,
                    y,
                    class,
                    escape_svg(&event.label)
                ));

                // Connecting line to axis
                svg.push_str(&format!(
                    r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{}" stroke="#ccc" stroke-dasharray="2,2"/>"##,
                    x, y + 5.0, x, axis_y
                ));
            }
        }
    }

    // Time labels
    if let (Some(start), Some(end)) = (timeline.start_time, timeline.end_time) {
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" class="event-label">{}</text>"#,
            margin,
            axis_y + 15,
            start.format("%H:%M:%S")
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="end" class="event-label">{}</text>"#,
            width - margin,
            axis_y + 15,
            end.format("%H:%M:%S")
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// Escape text for SVG.
fn escape_svg(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_event_creation() {
        let event = TimelineEvent::new(
            "test-1".to_string(),
            TimelineEventType::OperationStart,
            Utc::now(),
            "Test Operation".to_string(),
        )
        .with_description("Test description".to_string())
        .with_process_id("process-1".to_string())
        .with_duration_ms(100);

        assert_eq!(event.id, "test-1");
        assert!(event.description.is_some());
        assert!(event.process_id.is_some());
        assert_eq!(event.duration_ms, Some(100));
    }

    #[test]
    fn test_timeline_data_time_bounds() {
        let mut timeline = TimelineData::new("Test Timeline".to_string());

        let early = Utc::now() - ChronoDuration::hours(1);
        let late = Utc::now();

        timeline.add_event(TimelineEvent::new(
            "e1".to_string(),
            TimelineEventType::OperationStart,
            late,
            "Late".to_string(),
        ));

        timeline.add_event(TimelineEvent::new(
            "e2".to_string(),
            TimelineEventType::OperationEnd,
            early,
            "Early".to_string(),
        ));

        assert_eq!(timeline.start_time, Some(early));
        assert_eq!(timeline.end_time, Some(late));
    }

    #[test]
    fn test_timeline_sort() {
        let mut timeline = TimelineData::new("Test Timeline".to_string());

        let t1 = Utc::now();
        let t2 = t1 + ChronoDuration::seconds(1);
        let t3 = t1 + ChronoDuration::seconds(2);

        timeline.add_event(TimelineEvent::new(
            "e3".to_string(),
            TimelineEventType::OperationEnd,
            t3,
            "Third".to_string(),
        ));
        timeline.add_event(TimelineEvent::new(
            "e1".to_string(),
            TimelineEventType::OperationStart,
            t1,
            "First".to_string(),
        ));
        timeline.add_event(TimelineEvent::new(
            "e2".to_string(),
            TimelineEventType::FaultInjected,
            t2,
            "Second".to_string(),
        ));

        timeline.sort();

        assert_eq!(timeline.events[0].id, "e1");
        assert_eq!(timeline.events[1].id, "e2");
        assert_eq!(timeline.events[2].id, "e3");
    }

    #[test]
    fn test_timeline_generator() {
        let generator = TimelineGenerator::new()
            .with_operations(true)
            .with_faults(true)
            .with_violations(true)
            .with_max_events(100);

        let events = vec![
            TimelineEvent::new(
                "e1".to_string(),
                TimelineEventType::OperationStart,
                Utc::now(),
                "Op Start".to_string(),
            ),
            TimelineEvent::new(
                "e2".to_string(),
                TimelineEventType::FaultInjected,
                Utc::now(),
                "Fault".to_string(),
            ),
        ];

        let timeline = generator.generate_from_events(&events);

        assert_eq!(timeline.events.len(), 2);
    }

    #[test]
    fn test_timeline_to_svg() {
        let mut timeline = TimelineData::new("Test".to_string());
        timeline.start_time = Some(Utc::now());
        timeline.end_time = Some(Utc::now() + ChronoDuration::hours(1));

        timeline.add_event(TimelineEvent::new(
            "e1".to_string(),
            TimelineEventType::OperationStart,
            Utc::now() + ChronoDuration::minutes(30),
            "Test Event".to_string(),
        ));

        let svg = timeline_to_svg(&timeline, 800, 200);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Test"));
    }

    #[test]
    fn test_event_type_css_class() {
        assert_eq!(
            TimelineEventType::OperationStart.css_class(),
            "operation-start"
        );
        assert_eq!(
            TimelineEventType::ViolationDetected.css_class(),
            "violation-detected"
        );
        assert_eq!(
            TimelineEventType::Custom("test".to_string()).css_class(),
            "custom-event"
        );
    }
}
