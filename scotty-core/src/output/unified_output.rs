// Re-export types from scotty-types (TaskOutput is now defined there)
pub use scotty_types::{OutputLimits, OutputLine, OutputStreamType, TaskOutput};

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
        let mut output = TaskOutput::with_limits(OutputLimits {
            max_lines: 3,
            max_line_length: 100,
        });

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
        let mut output = TaskOutput::with_limits(OutputLimits {
            max_lines: 10,
            max_line_length: 20,
        });

        let long_line = "a".repeat(50);
        output.add_stdout(long_line);

        let stored_line = output.lines.last().unwrap();
        assert!(stored_line.content.len() <= 20);
        assert!(stored_line.content.ends_with("...[truncated]"));
    }

    #[test]
    fn test_task_output_stream_filtering() {
        let mut output = TaskOutput::new();

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
        let mut output = TaskOutput::new();

        for i in 1..=5 {
            output.add_stdout(format!("Line {}", i));
        }

        let recent = output.get_recent_lines(Some(3));
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "Line 3");
        assert_eq!(recent[1].content, "Line 4");
        assert_eq!(recent[2].content, "Line 5");
    }
}
