use crate::{context::AppContext, utils::ui::Ui};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Helper to simulate terminal output generation
async fn simulate_output(secs: u32, ui: &Arc<Ui>) {
    for i in 0..secs {
        ui.println(format!("{i}: Simulating output for {secs} seconds"));
        sleep(Duration::from_secs(1)).await;
    }
}

// Generates a lot of output to test scrolling behavior
async fn generate_large_output(ui: &Arc<Ui>) {
    ui.println("Generating large output to test scrolling...");
    for i in 0..20 {
        ui.println(format!(
            "Line {i} of test output - this is to verify the status line stays at the bottom"
        ));
        sleep(Duration::from_millis(100)).await;
    }
}

// Demonstrates a potential error scenario
fn simulate_error_condition(should_error: bool) -> anyhow::Result<String> {
    if should_error {
        Err(anyhow::anyhow!(
            "This is a simulated error to demonstrate the error handling in status line"
        ))
    } else {
        Ok("Operation completed successfully without errors".to_string())
    }
}

pub async fn run_tests(context: &AppContext) -> anyhow::Result<()> {
    // First test: Basic status line updates
    let ui = context.ui();
    ui.new_status_line("Step 1: Testing status line with standard output");
    ui.run(async || {
        // Demonstrate normal operation with throbber
        simulate_output(3, ui).await;

        // Update status line message during operation
        ui.new_status_line("Step 1B: Updated status message while running");
        simulate_output(2, ui).await;

        // Show success message
        ui.success("Step 1 completed successfully");

        // Start second phase
        ui.new_status_line("Step 2: Testing with large output");
        generate_large_output(ui).await;
        ui.success("Step 2 done - status line should remain visible at bottom");

        // Final test showing task completion
        Ok("Wow, we are done! Status line enhancement test complete.".to_string())
    })
    .await?;

    // Give a short pause between tests
    sleep(Duration::from_secs(1)).await;

    // Second test: Error handling
    let ui = context.ui();
    ui.new_status_line("Testing error handling in status line");
    if let Err(e) = ui
        .run(async || {
            simulate_output(2, ui).await;
            ui.new_status_line("About to encounter an error...");
            sleep(Duration::from_secs(1)).await;

            // This will trigger the error handling in StatusLine
            simulate_error_condition(true)
        })
        .await
    {
        println!("Error was handled as expected: {e}");
    }

    Ok(())
}
