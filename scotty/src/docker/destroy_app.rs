use std::sync::Arc;

use tokio::sync::RwLock;

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use crate::state_machine::StateHandler;
use crate::state_machine::StateMachine;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::app_data::AppStatus;
use scotty_core::notification_types::Message;
use scotty_core::notification_types::MessageType;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_sm;
use super::purge_app::purge_app_prepare;
use super::purge_app::PurgeAppMethod;
use super::state_machine_handlers::context::Context;
use super::state_machine_handlers::remove_directory_handler::RemoveDirectoryHandler;
use super::state_machine_handlers::set_finished_handler::SetFinishedHandler;
use super::state_machine_handlers::update_app_data_handler::UpdateAppDataHandler;

struct RunDockerComposeDownHandler<S> {
    next_state: S,
    app: AppData,
}

#[async_trait::async_trait]
impl StateHandler<DestroyAppStates, Context> for RunDockerComposeDownHandler<DestroyAppStates> {
    async fn transition(
        &self,
        _from: &DestroyAppStates,
        context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<DestroyAppStates> {
        let sm = purge_app_prepare(&self.app, PurgeAppMethod::Down).await?;
        let handle = sm.spawn(context.clone());
        let _ = handle.await;

        Ok(self.next_state)
    }
}

struct RemoveAppDataHandler {
    app_id: String,
    next_state: DestroyAppStates,
}

#[async_trait::async_trait]
impl StateHandler<DestroyAppStates, Context> for RemoveAppDataHandler {
    async fn transition(
        &self,
        _from: &DestroyAppStates,
        _context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<DestroyAppStates> {
        let app_state = _context.read().await.app_state.clone();
        app_state.apps.remove_app(&self.app_id).await?;

        Ok(self.next_state)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DestroyAppStates {
    RemoveDockerContainers,
    UpdateAppData,
    RemoveFilesAndDirectories,
    RemoveAppData,
    SetFinished,
    Done,
}

async fn destroy_app_prepare(
    app: &AppData,
) -> anyhow::Result<StateMachine<DestroyAppStates, Context>> {
    let mut sm = StateMachine::new(
        DestroyAppStates::RemoveDockerContainers,
        DestroyAppStates::Done,
    );

    sm.add_handler(
        DestroyAppStates::RemoveDockerContainers,
        Arc::new(RunDockerComposeDownHandler::<DestroyAppStates> {
            next_state: DestroyAppStates::UpdateAppData,
            app: app.clone(),
        }),
    );

    sm.add_handler(
        DestroyAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<DestroyAppStates> {
            next_state: DestroyAppStates::RemoveFilesAndDirectories,
        }),
    );

    sm.add_handler(
        DestroyAppStates::RemoveFilesAndDirectories,
        Arc::new(RemoveDirectoryHandler::<DestroyAppStates> {
            next_state: DestroyAppStates::RemoveAppData,
        }),
    );

    sm.add_handler(
        DestroyAppStates::RemoveAppData,
        Arc::new(RemoveAppDataHandler {
            next_state: DestroyAppStates::SetFinished,
            app_id: app.name.clone(),
        }),
    );

    sm.add_handler(
        DestroyAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<DestroyAppStates> {
            next_state: DestroyAppStates::Done,
            notification: Some(Message::new(MessageType::AppDestroyed, app)),
        }),
    );
    Ok(sm)
}

pub async fn destroy_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    let sm = destroy_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
