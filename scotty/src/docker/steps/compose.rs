use anyhow::Context as _;
use scotty_core::utils::secret::SecretHashMap;
use tracing::{info, instrument};

use crate::docker::find_apps::inspect_app;
use crate::tasks::manager::OutputMode;

use super::{
    context::{Context, Step},
    run_task_and_wait::run_task_and_wait,
};

/// `docker login` for the app's registry, if it has one.
#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn docker_login(ctx: &Context) -> anyhow::Result<()> {
    let Some(registry) = ctx.app_data.get_registry() else {
        ctx.skip_step(&Step::Login).await;
        return Ok(());
    };
    ctx.step(&Step::Login, login(ctx, &registry)).await
}

async fn login(ctx: &Context, registry: &str) -> anyhow::Result<()> {
    let registry = ctx
        .app_state
        .settings
        .docker
        .registries
        .get(registry)
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
        OutputMode::Plain,
    )
    .await
    .context("docker login")
}

/// `docker-compose <args>` with the app's environment, reported as the step
/// its subcommand stands for. A pull reports per-image progress, `up` also
/// the containers it started.
#[instrument(skip_all, fields(app = %ctx.app_data.name, args = ?args))]
pub async fn docker_compose(ctx: &Context, args: &[&str]) -> anyhow::Result<()> {
    let subcommand = args.first().copied().unwrap_or_default();
    let (mode, global_args): (_, &[&str]) = match subcommand {
        "pull" => (OutputMode::ComposeProgress, &["--progress", "json"]),
        "up" => (
            OutputMode::ComposeUp {
                services: ctx.app_data.services.len(),
            },
            &["--progress", "json"],
        ),
        _ => (OutputMode::Plain, &[]),
    };
    let full_args = [global_args, args].concat();
    let work = async {
        run_task_and_wait(
            ctx,
            &std::path::PathBuf::from(&ctx.app_data.docker_compose_path),
            "docker-compose",
            &full_args,
            &ctx.app_data.get_environment(),
            &format!("docker-compose {}", args.join(" ")),
            mode,
        )
        .await
        .with_context(|| format!("docker-compose {}", args.join(" ")))
    };
    let step = match subcommand {
        "pull" => Step::Pull,
        "build" => Step::Build,
        "stop" => Step::Stop,
        "up" => Step::Up,
        "down" | "rm" => Step::RemoveContainers,
        _ => return work.await,
    };
    ctx.step(&step, work).await
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
