use anyhow::Context as _;
use scotty_core::utils::secret::SecretHashMap;
use tracing::{info, instrument};

use crate::docker::find_apps::inspect_app;

use super::{context::Context, run_task_and_wait::run_task_and_wait};

/// `docker login` for the app's registry, if it has one.
#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn docker_login(ctx: &Context) -> anyhow::Result<()> {
    let Some(registry) = ctx.app_data.get_registry() else {
        return Ok(());
    };
    let registry = ctx
        .app_state
        .settings
        .docker
        .registries
        .get(&registry)
        .ok_or_else(|| anyhow::anyhow!("Registry {} not found in settings!", registry))?;
    let args = [
        "login",
        &registry.registry,
        "-u",
        &registry.username,
        "-p",
        registry.password.expose_secret(),
    ];
    run_task_and_wait(
        ctx,
        &std::path::PathBuf::from(&ctx.app_data.docker_compose_path),
        "docker",
        &args,
        &SecretHashMap::new(),
        &format!("Log into registry {}", registry.registry),
    )
    .await
    .context("docker login")
}

/// `docker-compose <args>` with the app's environment.
#[instrument(skip_all, fields(app = %ctx.app_data.name, args = ?args))]
pub async fn docker_compose(ctx: &Context, args: &[&str]) -> anyhow::Result<()> {
    run_task_and_wait(
        ctx,
        &std::path::PathBuf::from(&ctx.app_data.docker_compose_path),
        "docker-compose",
        args,
        &ctx.app_data.get_environment(),
        &format!("docker-compose {}", args.join(" ")),
    )
    .await
    .with_context(|| format!("docker-compose {}", args.join(" ")))
}

/// Re-inspect the app and store the result in the shared app list.
#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn update_app_data(ctx: &Context) -> anyhow::Result<()> {
    let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);
    info!(
        "Updating app from docker-compose file {}",
        docker_compose_path.display(),
    );
    let app_data = inspect_app(&ctx.app_state, &docker_compose_path).await?;
    ctx.app_state.apps.update_app(app_data).await?;
    Ok(())
}
