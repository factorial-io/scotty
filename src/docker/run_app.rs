use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    state_machine::state_machine::{StateHandler, StateMachine},
    tasks::{
        running_app_context::RunningAppContext,
        task_details::{State, TaskDetails},
    },
};

use super::{docker_compose::run_docker_compose, find_apps::inspect_app};

struct Context {
    app_state: SharedAppState,
    task: Arc<RwLock<TaskDetails>>,
    app_data: AppData,
}

impl Context {
    pub async fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: self.task.read().await.clone(),
            app_data: self.app_data.clone(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[derive(Debug)]
struct RunDockerComposeHandler;
#[derive(Debug)]
struct UpdateAppDataHandler;
#[derive(Debug)]
struct SetFinishedHandler;

#[async_trait::async_trait]
impl StateHandler<RunAppStates, Context> for RunDockerComposeHandler {
    #[instrument(skip(context))]
    async fn transition(
        &self,
        _from: &RunAppStates,
        context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<RunAppStates> {
        let context = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);
        debug!("Running docker-compose up -d");
        let task_details = run_docker_compose(
            &context.app_state,
            &docker_compose_path,
            &["up", "-d"],
            context.task.clone(),
        )
        .await?;

        let handle = context
            .app_state
            .task_manager
            .get_task_handle(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        debug!("Waiting for docker-compose up -d to finish");
        while !handle.read().await.is_finished() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        debug!("docker-compose up -d finished");

        Ok(RunAppStates::UpdateAppData)
    }
}

#[async_trait::async_trait]
impl StateHandler<RunAppStates, Context> for UpdateAppDataHandler {
    #[instrument(skip(context))]
    async fn transition(
        &self,
        _from: &RunAppStates,
        context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<RunAppStates> {
        let ctx = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);

        let app_data = inspect_app(&ctx.app_state, &docker_compose_path).await?;
        ctx.app_state.apps.update_app(app_data).await?;

        Ok(RunAppStates::SetFinished)
    }
}

#[async_trait::async_trait]
impl StateHandler<RunAppStates, Context> for SetFinishedHandler {
    #[instrument(skip(context))]
    async fn transition(
        &self,
        _from: &RunAppStates,
        context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<RunAppStates> {
        let context = context.read().await;
        let mut task_details = context.task.write().await;
        task_details.state = State::Finished;

        Ok(RunAppStates::Done)
    }
}
#[instrument(skip(app_state))]
pub async fn run_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(RunAppStates::RunDockerCompose, RunAppStates::Done);

    sm.add_handler(
        RunAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler),
    );
    sm.add_handler(RunAppStates::UpdateAppData, Arc::new(UpdateAppDataHandler));
    sm.add_handler(RunAppStates::SetFinished, Arc::new(SetFinishedHandler));
    let context = Arc::new(RwLock::new(Context {
        app_state: app_state.clone(),
        app_data: app.clone(),
        task: Arc::new(RwLock::new(TaskDetails::default())),
    }));

    let _ = sm.spawn(context.clone()).await;

    Ok(context.clone().read().await.as_running_app_context().await)
}
