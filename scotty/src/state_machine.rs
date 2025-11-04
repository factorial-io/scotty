use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

#[async_trait::async_trait]
pub trait StateHandler<S, C>
where
    S: Send + Sync,
{
    async fn transition(&self, from: &S, context: Arc<RwLock<C>>) -> anyhow::Result<S>;
}

pub struct StateMachine<S, C>
where
    S: Copy + PartialEq + Eq + std::hash::Hash + 'static + std::marker::Sync + std::marker::Send,
    C: std::marker::Sync + std::marker::Send + 'static,
{
    state: S,
    end_state: S,
    error_state: Option<S>,
    handlers: HashMap<S, Arc<dyn StateHandler<S, C> + Send + Sync>>,
}

impl<S, C> StateMachine<S, C>
where
    S: std::fmt::Debug
        + Copy
        + PartialEq
        + Eq
        + std::hash::Hash
        + 'static
        + std::marker::Sync
        + std::marker::Send,
    C: std::marker::Sync + std::marker::Send + 'static,
{
    pub fn new(initial_state: S, end_state: S) -> Self {
        Self {
            state: initial_state,
            end_state,
            error_state: None,
            handlers: HashMap::new(),
        }
    }

    pub fn set_error_state(&mut self, error_state: S) {
        self.error_state = Some(error_state);
    }

    pub fn add_handler(&mut self, state: S, handler: Arc<dyn StateHandler<S, C> + Send + Sync>) {
        self.handlers.insert(state, handler);
    }

    pub async fn run(&mut self, context: Arc<RwLock<C>>) -> anyhow::Result<()> {
        while self.state != self.end_state {
            if let Some(handler) = self.handlers.get(&self.state) {
                info!("Running handler for state {:?}", self.state);
                let old_state = self.state;
                match handler.transition(&self.state, context.clone()).await {
                    Ok(new_state) => {
                        self.state = new_state;
                        info!("Transitioned from {:?} to {:?}", old_state, self.state);
                    }
                    Err(e) => {
                        // If error_state is set, transition to it first
                        if let Some(error_state) = self.error_state {
                            error!(
                                "Handler for {:?} failed, transitioning to error state {:?}",
                                old_state, error_state
                            );
                            self.state = error_state;

                            // Run the error handler if it exists
                            if let Some(error_handler) = self.handlers.get(&error_state) {
                                let _ = error_handler
                                    .transition(&error_state, context.clone())
                                    .await;
                            }
                        }
                        return Err(e);
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "No handler found for state {:?}",
                    self.state
                ));
            }
        }
        Ok(())
    }

    pub fn spawn(self, context: Arc<RwLock<C>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let cloned_self = Arc::new(RwLock::new(self));

        // Spawn the main state machine task
        crate::metrics::spawn_instrumented(async move {
            let current_state = cloned_self.read().await.state;
            let result = cloned_self.write().await.run(context).await;

            // Log errors but also propagate them
            if let Err(ref e) = result {
                let failed_state = cloned_self.read().await.state;
                error!(
                    current_state = ?current_state,
                    failed_state = ?failed_state,
                    error = %e,
                    error_chain = ?e.chain().collect::<Vec<_>>(),
                    "State machine execution failed"
                );
            }

            result
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    enum TestState {
        Start,
        Middle,
        End,
    }

    struct TestHandler {
        next_state: TestState,
    }

    #[derive(Default)]
    pub struct Context {
        pub output: String,
    }

    #[async_trait::async_trait]
    impl StateHandler<TestState, Context> for TestHandler {
        async fn transition(
            &self,
            from: &TestState,
            context: Arc<RwLock<Context>>,
        ) -> anyhow::Result<TestState> {
            context
                .write()
                .await
                .output
                .push_str(&format!("{:?}->{:?}\n", from, self.next_state));
            Ok(self.next_state)
        }
    }

    #[tokio::test]
    async fn test_state_machine() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);

        let handler_start = Arc::new(TestHandler {
            next_state: TestState::Middle,
        });
        let handler_middle = Arc::new(TestHandler {
            next_state: TestState::End,
        });

        state_machine.add_handler(TestState::Start, handler_start);
        state_machine.add_handler(TestState::Middle, handler_middle);

        let context = Arc::new(RwLock::new(Context::default()));
        let result = state_machine.run(context.clone()).await;

        assert!(result.is_ok());
        assert_eq!(state_machine.state, TestState::End);
        assert_eq!(
            context.clone().read().await.output,
            "Start->Middle\nMiddle->End\n"
        )
    }

    /// Handler that always returns an error
    struct ErrorHandler {
        error_message: String,
    }

    #[async_trait::async_trait]
    impl StateHandler<TestState, Context> for ErrorHandler {
        async fn transition(
            &self,
            _from: &TestState,
            _context: Arc<RwLock<Context>>,
        ) -> anyhow::Result<TestState> {
            Err(anyhow::anyhow!("{}", self.error_message))
        }
    }

    /// Handler that panics
    struct PanicHandler;

    #[async_trait::async_trait]
    impl StateHandler<TestState, Context> for PanicHandler {
        async fn transition(
            &self,
            _from: &TestState,
            _context: Arc<RwLock<Context>>,
        ) -> anyhow::Result<TestState> {
            panic!("Intentional panic for testing");
        }
    }

    /// Test that errors from handlers propagate through run()
    #[tokio::test]
    async fn test_handler_error_propagates_through_run() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);

        let error_handler = Arc::new(ErrorHandler {
            error_message: "Test error".to_string(),
        });

        state_machine.add_handler(TestState::Start, error_handler);

        let context = Arc::new(RwLock::new(Context::default()));
        let result = state_machine.run(context).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Test error");
    }

    /// Test that errors from handlers propagate through spawn()
    #[tokio::test]
    async fn test_handler_error_propagates_through_spawn() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);

        let error_handler = Arc::new(ErrorHandler {
            error_message: "Spawn test error".to_string(),
        });

        state_machine.add_handler(TestState::Start, error_handler);

        let context = Arc::new(RwLock::new(Context::default()));
        let handle = state_machine.spawn(context);

        // Wait for the task to complete
        let result = handle.await;

        // Should get Ok(Err(...)) - task completed but returned error
        assert!(result.is_ok());
        let inner_result = result.unwrap();
        assert!(inner_result.is_err());
        assert_eq!(inner_result.unwrap_err().to_string(), "Spawn test error");
    }

    /// Test that error state is reached when handler fails
    #[tokio::test]
    async fn test_error_state_handler_runs() {
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        enum ErrorTestState {
            Start,
            ErrorState,
            End,
        }

        let mut state_machine = StateMachine::new(ErrorTestState::Start, ErrorTestState::End);
        state_machine.set_error_state(ErrorTestState::ErrorState);

        // Handler that fails
        struct FailingHandler;

        #[async_trait::async_trait]
        impl StateHandler<ErrorTestState, Context> for FailingHandler {
            async fn transition(
                &self,
                _from: &ErrorTestState,
                _context: Arc<RwLock<Context>>,
            ) -> anyhow::Result<ErrorTestState> {
                Err(anyhow::anyhow!("Handler failed"))
            }
        }

        // Error state handler that records it was called
        struct ErrorStateHandler;

        #[async_trait::async_trait]
        impl StateHandler<ErrorTestState, Context> for ErrorStateHandler {
            async fn transition(
                &self,
                _from: &ErrorTestState,
                context: Arc<RwLock<Context>>,
            ) -> anyhow::Result<ErrorTestState> {
                context
                    .write()
                    .await
                    .output
                    .push_str("Error handler called\n");
                Ok(ErrorTestState::End)
            }
        }

        state_machine.add_handler(ErrorTestState::Start, Arc::new(FailingHandler));
        state_machine.add_handler(ErrorTestState::ErrorState, Arc::new(ErrorStateHandler));

        let context = Arc::new(RwLock::new(Context::default()));
        let result = state_machine.run(context.clone()).await;

        // Should still return error even though error handler ran
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Handler failed");

        // Error handler should have been called
        assert_eq!(context.read().await.output, "Error handler called\n");
    }

    /// Test that panics are caught by spawn() and returned as JoinError
    #[tokio::test]
    async fn test_handler_panic_caught_by_spawn() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);
        state_machine.add_handler(TestState::Start, Arc::new(PanicHandler));

        let context = Arc::new(RwLock::new(Context::default()));
        let handle = state_machine.spawn(context);

        // Wait for the task to complete
        let result = handle.await;

        // Should get Err(JoinError) when task panics
        assert!(result.is_err());
        let join_error = result.unwrap_err();
        assert!(join_error.is_panic());
    }

    /// Test idiomatic error handling pattern used in nested state machines
    #[tokio::test]
    async fn test_idiomatic_error_handling() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);

        let error_handler = Arc::new(ErrorHandler {
            error_message: "Inner error".to_string(),
        });

        state_machine.add_handler(TestState::Start, error_handler);

        let context = Arc::new(RwLock::new(Context::default()));
        let handle = state_machine.spawn(context);

        // Test the idiomatic pattern: handle.await.map_err(...)??.context(...)?
        let result: anyhow::Result<()> = handle
            .await
            .map_err(|e| anyhow::anyhow!("Task panicked: {}", e))
            .and_then(|r| r.map_err(|e| e.context("Outer context")));

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        // Should have both the context and the original error in the chain
        // Using Debug format to see the full error chain
        assert!(error_msg.contains("Outer context") || error.to_string().contains("Outer context"));
        assert!(error_msg.contains("Inner error") || error.to_string().contains("Inner error"));
    }

    /// Test idiomatic panic handling pattern
    #[tokio::test]
    async fn test_idiomatic_panic_handling() {
        let mut state_machine = StateMachine::new(TestState::Start, TestState::End);
        state_machine.add_handler(TestState::Start, Arc::new(PanicHandler));

        let context = Arc::new(RwLock::new(Context::default()));
        let handle = state_machine.spawn(context);

        // Test the idiomatic pattern with panic
        let result: anyhow::Result<()> = handle
            .await
            .map_err(|e| anyhow::anyhow!("Task panicked: {}", e))
            .and_then(|r| r.map_err(|e| e.context("Outer context")));

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should indicate it was a panic
        assert!(error_msg.contains("Task panicked"));
    }
}
