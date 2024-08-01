use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::docker_compose::run_docker_compose,
    state_machine::StateHandler,
    tasks::{
        running_app_context::RunningAppContext,
        task_details::{State, TaskDetails},
    },
};

use super::find_apps::inspect_app;

pub struct Context {
    pub app_state: SharedAppState,
    pub task: Arc<RwLock<TaskDetails>>,
    pub app_data: AppData,
}

impl Context {
    pub async fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: self.task.read().await.clone(),
            app_data: self.app_data.clone(),
        }
    }

    pub fn create(app_state: SharedAppState, app_data: &AppData) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Context {
            app_state: app_state.clone(),
            app_data: app_data.clone(),
            task: Arc::new(RwLock::new(TaskDetails::default())),
        }))
    }
}

#[derive(Debug)]
pub struct RunDockerComposeHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub command: Vec<String>,
}

#[derive(Debug)]
pub struct UpdateAppDataHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[derive(Debug)]
pub struct SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for RunDockerComposeHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);
        debug!("Running docker-compose {}", &self.command.join(" "));
        let task_details = run_docker_compose(
            &context.app_state,
            &docker_compose_path,
            self
                .command
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
            context.task.clone(),
        )
        .await?;

        let handle = context
            .app_state
            .task_manager
            .get_task_handle(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        debug!(
            "Waiting for docker-compose {} to finish",
            &self.command.join(" ")
        );
        while !handle.read().await.is_finished() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        debug!("docker-compose {} finished", &self.command.join(" "));

        Ok(self.next_state.clone())
    }
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for UpdateAppDataHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let ctx = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);

        let app_data = inspect_app(&ctx.app_state, &docker_compose_path).await?;
        ctx.app_state.apps.update_app(app_data).await?;

        Ok(self.next_state.clone())
    }
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let mut task_details = context.task.write().await;
        task_details.state = State::Finished;

        Ok(self.next_state.clone())
    }
}
