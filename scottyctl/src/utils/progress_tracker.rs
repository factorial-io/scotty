use crossterm::{
    cursor,
    execute,
    terminal::{Clear, ClearType},
};
use owo_colors::OwoColorize;
use std::io::{self, stdout, Write};
use std::time::Instant;

/// ProgressTracker shows operation progress while preserving normal stdout functionality.
///
/// It works by maintaining a status line at the bottom of the terminal output while
/// ensuring normal stdout appears above it.
pub struct ProgressTracker {
    active: bool,
    message: String,
    start_time: Instant,
}

impl ProgressTracker {
    /// Creates a new progress tracker
    pub fn new() -> Self {
        Self {
            active: false,
            message: String::new(),
            start_time: Instant::now(),
        }
    }

    /// Starts tracking a new operation with the given message
    pub fn start_operation(&mut self, message: &str) -> io::Result<()> {
        // If there's an active operation, make it part of history first
        if self.active {
            writeln!(stdout())?;
        }

        // Update state
        self.message = message.to_string();
        self.start_time = Instant::now();
        self.active = true;

        // Print initial status line
        let mut stdout = stdout();
        write!(stdout, "- {} ... ", message)?;
        stdout.flush()?;

        Ok(())
    }

    /// Marks the current operation as complete with a success message
    pub fn complete_operation(&mut self, result: &str) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        self.active = false;
        
        // Clear current line and rewrite with completion status
        let mut stdout = stdout();
        execute!(stdout, Clear(ClearType::CurrentLine), cursor::MoveToColumn(0))?;
        
        write!(stdout, "✓ {}", result.green())?;
        stdout.flush()?;
        
        Ok(())
    }

    /// Marks the current operation as failed with an error message
    pub fn fail_operation(&mut self, error: &str) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        self.active = false;
        
        // Clear current line and rewrite with error status
        let mut stdout = stdout();
        execute!(stdout, Clear(ClearType::CurrentLine), cursor::MoveToColumn(0))?;
        
        write!(stdout, "✗ {}", error.red())?;
        stdout.flush()?;
        
        Ok(())
    }

    /// Print a message to stdout while preserving the progress display
    pub fn print(&self, message: &str) -> io::Result<()> {
        if !self.active {
            // No active progress, just print normally
            print!("{}", message);
            return io::stdout().flush();
        }

        let mut stdout = stdout();
        
        // Save current line with progress indicator
        let current_progress = format!("- {} ... ", self.message);
        
        // Clear the line with progress indicator
        execute!(stdout, Clear(ClearType::CurrentLine), cursor::MoveToColumn(0))?;
        
        // Print the message
        write!(stdout, "{}", message)?;
        
        // Redraw the progress indicator
        write!(stdout, "{}", current_progress)?;
        stdout.flush()?;
        
        Ok(())
    }

    /// Print a line to stdout while preserving the progress display
    pub fn println(&self, message: &str) -> io::Result<()> {
        if !self.active {
            // No active progress, just print normally
            println!("{}", message);
            return Ok(());
        }

        let mut stdout = stdout();
        
        // Clear the line with progress indicator
        execute!(stdout, Clear(ClearType::CurrentLine), cursor::MoveToColumn(0))?;
        
        // Print the message with newline
        writeln!(stdout, "{}", message)?;
        
        // Redraw the progress indicator
        write!(stdout, "- {} ... ", self.message)?;
        stdout.flush()?;
        
        Ok(())
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if self.active {
            let _ = self.complete_operation(&format!("{} (completed)", self.message));
        }
    }
}

