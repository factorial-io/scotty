use crate::{utils::ui::StatusLine, ServerSettings};
use std::time::Duration;

// Helper to simulate terminal output generation
fn simulate_output(secs: u32) {
    for i in 0..secs {
        println!("{}: Simulating output for {} seconds", i, secs);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

// Generates a lot of output to test scrolling behavior
fn generate_large_output() {
    println!("Generating large output to test scrolling...");
    for i in 0..20 {
        println!("Line {} of test output - this is to verify the status line stays at the bottom", i);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// Demonstrates a potential error scenario
fn simulate_error_condition(should_error: bool) -> anyhow::Result<String> {
    if should_error {
        Err(anyhow::anyhow!("This is a simulated error to demonstrate the error handling in status line"))
    } else {
        Ok("Operation completed successfully without errors".to_string())
    }
}

pub async fn run_tests(_server_settings: &ServerSettings) -> anyhow::Result<()> {
    // First test: Basic status line updates
    let status = StatusLine::new("Step 1: Testing status line with standard output");
    status
        .run(async || {
            // Demonstrate normal operation with throbber
            simulate_output(3);
            
            // Update status line message during operation
            status.new_status_line("Step 1B: Updated status message while running");
            simulate_output(2);
            
            // Show success message
            status.success("Step 1 completed successfully");
            
            // Start second phase
            status.new_status_line("Step 2: Testing with large output");
            generate_large_output();
            status.success("Step 2 done - status line should remain visible at bottom");
            
            // Final test showing task completion
            Ok("Wow, we are done! Status line enhancement test complete.".to_string())
        })
        .await?;
    
    // Give a short pause between tests
    std::thread::sleep(Duration::from_secs(1));
    
    // Second test: Error handling
    let error_status = StatusLine::new("Testing error handling in status line");
    if let Err(e) = error_status
        .run(async || {
            simulate_output(2);
            error_status.new_status_line("About to encounter an error...");
            std::thread::sleep(Duration::from_secs(1));
            
            // This will trigger the error handling in StatusLine
            simulate_error_condition(true)
        })
        .await
    {
        println!("Error was handled as expected: {}", e);
    }
    
    Ok(())
}
