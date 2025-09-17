use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

/// Represents the type of output stream
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum OutputStreamType {
    Stdout,
    Stderr,
}

impl std::fmt::Display for OutputStreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputStreamType::Stdout => write!(f, "stdout"),
            OutputStreamType::Stderr => write!(f, "stderr"),
        }
    }
}

/// A single line of output with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct OutputLine {
    /// Timestamp when the line was received
    pub timestamp: DateTime<Utc>,
    /// Type of stream (stdout or stderr)
    pub stream: OutputStreamType,
    /// The actual content of the line
    pub content: String,
    /// Sequence number for ordering guarantee
    pub sequence: u64,
}

impl OutputLine {
    pub fn new(stream: OutputStreamType, content: String, sequence: u64) -> Self {
        Self {
            timestamp: Utc::now(),
            stream,
            content,
            sequence,
        }
    }

    pub fn stdout(content: String, sequence: u64) -> Self {
        Self::new(OutputStreamType::Stdout, content, sequence)
    }

    pub fn stderr(content: String, sequence: u64) -> Self {
        Self::new(OutputStreamType::Stderr, content, sequence)
    }
}

/// Configuration for output collection limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLimits {
    /// Maximum number of lines to keep in memory
    pub max_lines: usize,
    /// Maximum length of a single line (characters)
    pub max_line_length: usize,
}

impl Default for OutputLimits {
    fn default() -> Self {
        Self {
            max_lines: 10000,
            max_line_length: 4096,
        }
    }
}

/// Unified output collection for tasks with time-synchronized streams
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TaskOutput {
    /// ID of the associated task
    pub task_id: Uuid,
    /// Collected output lines in chronological order
    #[cfg_attr(feature = "utoipa", schema(value_type = Vec<OutputLine>))]
    pub lines: VecDeque<OutputLine>,
    /// Configuration limits
    #[serde(skip)]
    pub limits: OutputLimits,
    /// Total number of lines that have been processed (including evicted ones)
    pub total_lines_processed: u64,
    /// Current sequence number for new lines
    #[serde(skip)]
    pub current_sequence: u64,
}

impl TaskOutput {
    pub fn new(task_id: Uuid) -> Self {
        Self::with_limits(task_id, OutputLimits::default())
    }

    pub fn new_with_settings(task_id: Uuid, settings: &crate::settings::output::OutputSettings) -> Self {
        let limits = OutputLimits {
            max_lines: settings.max_lines,
            max_line_length: settings.max_line_length,
        };
        Self::with_limits(task_id, limits)
    }

    pub fn with_limits(task_id: Uuid, limits: OutputLimits) -> Self {
        Self {
            task_id,
            lines: VecDeque::with_capacity(limits.max_lines),
            limits,
            total_lines_processed: 0,
            current_sequence: 0,
        }
    }

    /// Add a new output line, maintaining size limits
    pub fn add_line(&mut self, stream: OutputStreamType, content: String) {
        // Truncate content if it exceeds the limit
        let truncated_content = if content.len() > self.limits.max_line_length {
            format!("{}... [TRUNCATED]", &content[..self.limits.max_line_length - 15])
        } else {
            content
        };

        let line = OutputLine::new(stream, truncated_content, self.current_sequence);
        self.current_sequence += 1;
        self.total_lines_processed += 1;

        self.lines.push_back(line);

        // Enforce line limit by removing oldest lines
        while self.lines.len() > self.limits.max_lines {
            self.lines.pop_front();
        }
    }

    /// Add stdout line
    pub fn add_stdout(&mut self, content: String) {
        self.add_line(OutputStreamType::Stdout, content);
    }

    /// Add stderr line
    pub fn add_stderr(&mut self, content: String) {
        self.add_line(OutputStreamType::Stderr, content);
    }

    /// Get the most recent N lines
    pub fn get_recent_lines(&self, count: usize) -> Vec<&OutputLine> {
        self.lines
            .iter()
            .rev()
            .take(count)
            .rev()
            .collect()
    }

    /// Get lines by stream type
    pub fn get_lines_by_stream(&self, stream_type: OutputStreamType) -> Vec<&OutputLine> {
        self.lines
            .iter()
            .filter(|line| line.stream == stream_type)
            .collect()
    }

    /// Get total number of lines currently in memory
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Check if any lines have been evicted due to limits
    pub fn has_truncated_history(&self) -> bool {
        self.total_lines_processed > self.lines.len() as u64
    }

    /// Get the timestamp of the oldest line in memory
    pub fn oldest_line_timestamp(&self) -> Option<DateTime<Utc>> {
        self.lines.front().map(|line| line.timestamp)
    }

    /// Get the timestamp of the newest line in memory
    pub fn newest_line_timestamp(&self) -> Option<DateTime<Utc>> {
        self.lines.back().map(|line| line.timestamp)
    }

    /// Clear all output lines
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_line_creation() {
        let line = OutputLine::stdout("Hello, world!".to_string(), 1);
        assert_eq!(line.stream, OutputStreamType::Stdout);
        assert_eq!(line.content, "Hello, world!");
        assert_eq!(line.sequence, 1);
    }

    #[test]
    fn test_task_output_line_limits() {
        let mut output = TaskOutput::with_limits(
            Uuid::new_v4(),
            OutputLimits {
                max_lines: 3,
                max_line_length: 100,
            },
        );

        output.add_stdout("Line 1".to_string());
        output.add_stdout("Line 2".to_string());
        output.add_stdout("Line 3".to_string());
        assert_eq!(output.line_count(), 3);

        // Adding a 4th line should evict the first
        output.add_stdout("Line 4".to_string());
        assert_eq!(output.line_count(), 3);
        assert_eq!(output.total_lines_processed, 4);
        assert!(output.has_truncated_history());

        // Check that Line 1 was evicted
        let lines: Vec<_> = output.lines.iter().map(|l| &l.content).collect();
        assert_eq!(lines, vec!["Line 2", "Line 3", "Line 4"]);
    }

    #[test]
    fn test_task_output_line_length_limits() {
        let mut output = TaskOutput::with_limits(
            Uuid::new_v4(),
            OutputLimits {
                max_lines: 10,
                max_line_length: 20,
            },
        );

        let long_line = "a".repeat(50);
        output.add_stdout(long_line);

        let stored_line = output.lines.back().unwrap();
        assert!(stored_line.content.len() <= 20);
        assert!(stored_line.content.ends_with("... [TRUNCATED]"));
    }

    #[test]
    fn test_task_output_stream_filtering() {
        let mut output = TaskOutput::new(Uuid::new_v4());

        output.add_stdout("stdout line 1".to_string());
        output.add_stderr("stderr line 1".to_string());
        output.add_stdout("stdout line 2".to_string());

        let stdout_lines = output.get_lines_by_stream(OutputStreamType::Stdout);
        let stderr_lines = output.get_lines_by_stream(OutputStreamType::Stderr);

        assert_eq!(stdout_lines.len(), 2);
        assert_eq!(stderr_lines.len(), 1);
        assert_eq!(stdout_lines[0].content, "stdout line 1");
        assert_eq!(stderr_lines[0].content, "stderr line 1");
    }

    #[test]
    fn test_task_output_recent_lines() {
        let mut output = TaskOutput::new(Uuid::new_v4());

        for i in 1..=5 {
            output.add_stdout(format!("Line {}", i));
        }

        let recent = output.get_recent_lines(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "Line 3");
        assert_eq!(recent[1].content, "Line 4");
        assert_eq!(recent[2].content, "Line 5");
    }
}