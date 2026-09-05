use anyhow::Context as _;
use scotty_core::utils::slugify::slugify;
use tracing::info;

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::app_data::AppSettings;
use scotty_core::apps::file_list::{File, FileList};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::settings::app_blueprint::ActionName;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_operation;
use super::rebuild_app::rebuild_steps;
use super::steps::compose::update_app_data;
use super::steps::context::Context;
use super::steps::files::{create_directory, save_files, save_settings};
use super::steps::load_balancer::create_load_balancer_config;
use super::steps::post_actions::run_post_actions;
use super::validation::validate_docker_compose_content;

pub async fn create_steps(
    ctx: &Context,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<()> {
    create_directory(ctx).await?;
    save_settings(ctx, settings).await?;
    save_files(ctx, files).await?;
    create_load_balancer_config(ctx, settings).await?;
    // The per-app network is ensured inside rebuild_steps before any compose
    // command, so create does not ensure it separately.
    rebuild_steps(ctx, false)
        .await
        .context("Docker compose rebuild failed")?;
    run_post_actions(ctx, &ActionName::PostCreate).await?;
    update_app_data(ctx).await
}

async fn validate_app(
    app_state: SharedAppState,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<File> {
    let docker_compose_file = files
        .files
        .iter()
        .find(|f| scotty_core::utils::compose::is_valid_config_file(&f.name));

    if docker_compose_file.is_none() {
        return Err(AppError::NoDockerComposeFile.into());
    }
    // Parse docker-compose file
    let docker_compose_content = docker_compose_file.unwrap().content.clone();

    // Create a vector with all the public service names
    let public_service_names: Vec<String> = settings
        .public_services
        .iter()
        .map(|service| service.service.clone())
        .collect();

    // Validation only checks if keys exist, not values - pass SecretHashMap directly
    let available_services = validate_docker_compose_content(
        &docker_compose_content,
        &public_service_names,
        Some(&settings.environment),
    )?;
    // Check if we know about the private registry.
    if let Some(registry) = &settings.registry {
        if !app_state.settings.docker.registries.contains_key(registry) {
            return Err(AppError::RegistryNotFound(registry.clone()).into());
        }
    }

    if let Some(app_blueprint) = &settings.app_blueprint {
        if !app_state
            .settings
            .apps
            .blueprints
            .contains_key(app_blueprint)
        {
            return Err(AppError::AppBlueprintNotFound(app_blueprint.clone()).into());
        }

        let app_blueprint = &app_state.settings.apps.blueprints[app_blueprint];

        // Check if docker-compose services match required services
        let required_services = &app_blueprint.required_services;
        let missing_services: Vec<String> = required_services
            .iter()
            .filter(|service| !available_services.contains(service))
            .cloned()
            .collect();

        if !missing_services.is_empty() {
            return Err(AppError::AppBlueprintMismatch(format!(
                "docker compose does not contain all required services: {missing_services:?}",
            ))
            .into());
        }
    }

    // Validate that all specified groups exist in the authorization system
    if let Err(missing_scopes) = app_state
        .auth_service
        .validate_scopes(&settings.scopes)
        .await
    {
        return Err(AppError::ScopesNotFound(missing_scopes).into());
    }

    Ok(docker_compose_file.unwrap().clone())
}

/// Asynchronously creates a new application by validating input files, preparing application data, and running the creation steps.
///
/// This function validates the provided Docker Compose files and settings, constructs the necessary application directories and metadata, and executes the stepwise creation workflow. Returns a context representing the running application upon successful creation.
///
/// # Errors
///
/// Returns an error if validation fails, required files are missing, or any step in the creation workflow encounters an error.
///
/// # Examples
///
/// ```no_run
/// # use scotty::docker::create_app::create_app;
/// # async fn example() {
/// # let app_state = todo!();
/// # let settings = todo!();
/// # let files = todo!();
/// let result = create_app(app_state, "my-app", &settings, &files).await;
/// assert!(result.is_ok());
/// # }
/// ```
pub async fn create_app(
    app_state: SharedAppState,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<RunningAppContext> {
    info!("Creating app: {}", app_name);
    let candidate = validate_app(app_state.clone(), settings, files).await?;
    let root_directory = app_state.settings.apps.root_folder.clone();
    let app_folder = slugify(app_name);
    let root_directory = format!("{root_directory}/{app_folder}");

    let docker_compose_path = format!("{}/{}", root_directory, candidate.name);
    let app_data = AppData {
        name: app_name.to_string(),
        settings: Some(settings.clone()),
        services: vec![],
        docker_compose_path,
        root_directory,
        status: scotty_core::apps::app_data::AppStatus::Creating,
        last_checked: None,
        // Never swept, so there is no next-check time to report yet.
        next_check: None,
        // The app does not exist yet; the reconciler fills this in once it has
        // observed the app's proxy network.
        load_balancer_connectivity: Default::default(),
    };
    let notification = Message::new(MessageType::AppCreated, &app_data);
    let settings = settings.clone();
    let files = files.clone();
    run_operation(
        app_state,
        &app_data,
        Some(notification),
        move |ctx| async move { create_steps(&ctx, &settings, &files).await },
    )
    .await
}
