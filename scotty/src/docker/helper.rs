use crate::{app_state::SharedAppState, state_machine::StateMachine};
use scotty_core::apps::app_data::AppData;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::state_machine_handlers::context::Context;

pub async fn run_sm<S>(
    app_state: SharedAppState,
    app: &AppData,
    sm: StateMachine<S, Context>,
) -> anyhow::Result<RunningAppContext>
where
    S: Copy
        + PartialEq
        + Eq
        + std::hash::Hash
        + 'static
        + std::marker::Sync
        + std::marker::Send
        + std::fmt::Debug,
{
    let context = Context::create(app_state, app);
    {
        let context = context.write().await;
        let task = context.task.clone();
        context
            .app_state
            .task_manager
            .add_task(&task.read().await.id, task.clone(), None)
            .await;
    }
    let _handle = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
