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
            handlers: HashMap::new(),
        }
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

    pub fn spawn(self, context: Arc<RwLock<C>>) -> tokio::task::JoinHandle<()> {
        let cloned_self = Arc::new(RwLock::new(self));
        tokio::spawn(async move {
            let current_state = cloned_self.read().await.state;
            if let Err(e) = cloned_self.write().await.run(context).await {
                let failed_state = cloned_self.read().await.state;
                error!(
                    current_state = ?current_state,
                    failed_state = ?failed_state,
                    error = %e,
                    error_chain = ?e.chain().collect::<Vec<_>>(),
                    "State machine execution failed"
                );
            }
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
}
