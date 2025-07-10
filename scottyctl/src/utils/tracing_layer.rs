use crate::utils::ui::Ui;
use chrono::Local;
use owo_colors::OwoColorize;
use std::sync::Arc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// A custom tracing layer that routes log messages through the UI component
pub struct UiLayer {
    ui: Arc<Ui>,
}

impl UiLayer {
    /// Create a new UI layer with the given UI component
    pub fn new(ui: Arc<Ui>) -> Self {
        UiLayer { ui }
    }
}

/// Helper function to colorize the log level
fn colorize_level(level: &Level) -> String {
    match *level {
        Level::ERROR => level.to_string().bright_red().to_string(),
        Level::WARN => level.to_string().yellow().to_string(),
        Level::INFO => level.to_string().green().to_string(),
        Level::DEBUG => level.to_string().bright_blue().to_string(),
        Level::TRACE => level.to_string().dimmed().to_string(),
    }
}

impl<S> Layer<S> for UiLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Extract the log message
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        // Get metadata
        let level = event.metadata().level();
        let target = event.metadata().target();
        let file = event.metadata().file().unwrap_or("<unknown>");
        let line = event.metadata().line().unwrap_or(0);

        // Get current timestamp
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");

        // Format the message with timestamp, level, etc.
        let formatted_msg = format!(
            "{} {} [{}] [{}:{}] {}",
            timestamp.to_string().dimmed(),
            colorize_level(level),
            target.cyan(),
            file.dimmed(),
            line.to_string().dimmed(),
            message
        );

        // Use the UI's println/eprintln based on level
        match level {
            &Level::ERROR | &Level::WARN => {
                self.ui.eprintln(&formatted_msg);
            }
            _ => {
                // For info, debug, and trace levels
                self.ui.println(&formatted_msg);
            }
        }
    }
}

/// Visitor to extract the message from the event
struct MessageVisitor<'a>(&'a mut String);

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // For most debug-formatted values
        if field.name() == "message" {
            let debug_str = format!("{value:?}");
            // Remove quotes if the debug output is a simple string with quotes
            if debug_str.starts_with('"') && debug_str.ends_with('"') && debug_str.len() > 2 {
                self.0.push_str(&debug_str[1..debug_str.len() - 1]);
            } else {
                self.0.push_str(&debug_str);
            }
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        // For string values
        if field.name() == "message" {
            self.0.push_str(value);
        }
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if field.name() == "message" {
            self.0.push_str(&value.to_string());
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if field.name() == "message" {
            self.0.push_str(&value.to_string());
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "message" {
            self.0.push_str(&value.to_string());
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "message" {
            self.0.push_str(&value.to_string());
        }
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if field.name() == "message" {
            self.0.push_str(&value.to_string());
        }
    }
}
