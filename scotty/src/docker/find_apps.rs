use std::{collections::HashMap, path::PathBuf};

use bollard::container::InspectContainerOptions;
use chrono::{DateTime, Local};
use futures_util::future::join_all;
use scotty_core::apps::{
    app_data::{AppData, AppSettings, AppStatus, ContainerState},
    shared_app_list::AppDataVec,
};
use serde_yml::Value;
use std::path::Path;
use tokio::task;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, instrument, Instrument};
use walkdir::WalkDir;

use crate::{app_state::SharedAppState, docker::docker_compose::run_docker_compose_now};

use super::{
    loadbalancer::factory::LoadBalancerFactory, validation::validate_docker_compose_content,
};

type PathBufVec = Vec<PathBuf>;

#[instrument(skip(app_state))]
pub async fn find_apps(app_state: &SharedAppState) -> anyhow::Result<AppDataVec> {
    let mut paths = vec![];
    traverse_directory(app_state, &mut paths).await?;

    tracing::info!("Found {} potential app directories", paths.len());
    tracing::info!("{:?}", paths);

    // Vector to hold the join handles of the spawned tasks
    let mut handles = vec![];

    for path in paths {
        let app_state = app_state.clone();
        let handle = task::spawn(
            async move {
                let app_state = app_state.clone();

                match inspect_app(&app_state, &path).await {
                    Ok(app) => Ok(app),
                    Err(e) => {
                        error!("Error inspecting app at {}: {}", &path.display(), e);
                        Err(e)
                    }
                }
            }
            .instrument(tracing::info_span!("inspect_app task")),
        );
        handles.push(handle);
    }

    // Await all tasks to complete and collect results
    let results: Vec<Result<AppData, anyhow::Error>> = join_all(handles)
        .await
        .into_iter()
        .map(|handle| handle.unwrap())
        .collect();

    // Handle the results
    let apps = results.into_iter().filter_map(Result::ok).collect();

    Ok(AppDataVec { apps })
}

#[instrument()]
async fn extract_services_from_docker_compose(content: &str) -> anyhow::Result<Vec<String>> {
    let yaml: Value = serde_yml::from_str(content)?;

    let services = yaml
        .get("services")
        .ok_or_else(|| anyhow::anyhow!("No services found in docker-compose file"))?
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("Invalid services format in docker-compose file"))?
        .keys()
        .filter_map(|key| key.as_str().map(String::from))
        .collect();

    Ok(services)
}

#[instrument(skip(app_state))]
pub async fn inspect_app(
    app_state: &SharedAppState,
    docker_compose_path: &PathBuf,
) -> anyhow::Result<AppData> {
    let app_path = docker_compose_path.parent().unwrap();
    if app_path == Path::new(&app_state.settings.apps.root_folder) {
        return Err(anyhow::anyhow!(
            "Apps in the root paths are not supported, ignoring."
        ));
    }

    let name = app_path
        .strip_prefix(&app_state.settings.apps.root_folder)?
        .components()
        .map(|comp| comp.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("--");

    let content = std::fs::read_to_string(docker_compose_path)?;
    let dc_services = extract_services_from_docker_compose(&content).await?;
    let services =
        get_running_services(app_state, docker_compose_path, &name, &dc_services).await?;

    let settings = match get_app_settings(docker_compose_path).await {
        Ok(result) => result,
        Err(err) => {
            error!("{}", err);
            None
        }
    };
    let valid_settings = settings.is_some();

    let mut app_data = AppData::new(
        &name,
        app_path.to_str().unwrap(),
        docker_compose_path.to_str().unwrap(),
        services,
        settings,
    );

    // Set the last_checked timestamp to the current time
    app_data.last_checked = Some(chrono::Local::now());

    if !valid_settings
        && validate_docker_compose_content(content.as_bytes(), &dc_services, None).is_err()
    {
        app_data.status = AppStatus::Unsupported;
    }
    Ok(app_data)
}

#[instrument()]
async fn get_app_settings(docker_compose_path: &PathBuf) -> anyhow::Result<Option<AppSettings>> {
    let settings_path = docker_compose_path.with_file_name(".scotty.yml");
    AppSettings::from_file(&settings_path)
}

#[instrument(skip(app_state))]
async fn get_running_services(
    app_state: &SharedAppState,
    docker_compose_path: &PathBuf,
    app_name: &str,
    service_names: &Vec<String>,
) -> anyhow::Result<Vec<ContainerState>> {
    let running_containers = inspect_docker_compose(app_state, docker_compose_path).await?;
    let mut running_services: HashMap<String, ContainerState> = HashMap::new();

    for item in running_containers {
        running_services.insert(item.service.clone(), item);
    }

    let services: Vec<_> = service_names
        .iter()
        .map(|s| {
            if let Some(container_state) = running_services.get(s) {
                container_state.clone()
            } else {
                ContainerState {
                    status: scotty_core::apps::app_data::ContainerStatus::Empty,
                    id: None,
                    service: s.to_string(),
                    domains: vec![],
                    use_tls: false,
                    port: None,
                    started_at: None,
                    used_registry: None,
                    basic_auth: None,
                }
            }
        })
        .collect();

    info!("Services for app {}: {:?}", app_name, services);

    Ok(services)
}

#[instrument(skip(state, result))]
async fn traverse_directory(state: &SharedAppState, result: &mut PathBufVec) -> anyhow::Result<()> {
    let settings = &state.settings.apps;
    info!("Starting directory traversal with settings: {:?}", settings);
    for entry in WalkDir::new(&settings.root_folder).max_depth(settings.max_depth as usize) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            match entry.file_name().to_str().unwrap() {
                "docker-compose.yml" => result.push(path),
                "docker-compose.yaml" => result.push(path),
                _ => (),
            }
        }

        debug!(path = %entry.path().display(), "Visited");
    }

    Ok(())
}

#[instrument(skip(state))]
async fn inspect_docker_compose(
    state: &SharedAppState,
    file: &PathBuf,
) -> anyhow::Result<Vec<ContainerState>> {
    let output = run_docker_compose_now(file, &["ps", "-q", "-a"], None, false)?;
    let containers: Vec<String> = output.lines().map(String::from).collect();
    info!(
        "Found containers for {}: {}",
        &file.display(),
        containers.join(", ")
    );
    let mut stream = tokio_stream::iter(containers);

    let mut container_states = vec![];
    while let Some(item) = stream.next().await {
        async {
            match inspect_docker_container(state, &item).await {
                Ok(container_state) => container_states.push(container_state),
                Err(e) => {
                    error!(
                        "Failed to inspect container {} for file {}: {}",
                        &item,
                        &file.display(),
                        e
                    );
                }
            }
        }
        .instrument(tracing::info_span!("inspect_docker_container loop"))
        .await;
    }
    Ok(container_states)
}

#[instrument(skip(app_state))]
async fn inspect_docker_container(
    app_state: &SharedAppState,
    container_id: &str,
) -> anyhow::Result<ContainerState> {
    let insights = app_state
        .docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await?;

    let state = insights.state.clone().unwrap();
    let started_at_str = state.started_at.unwrap();
    let parsed_date =
        DateTime::parse_from_rfc3339(&started_at_str).expect("Failed to parse datetime");

    // Convert the UTC DateTime to local time
    let local_date: DateTime<Local> = parsed_date.with_timezone(&Local);

    let labels = insights.config.clone().unwrap().labels.unwrap();
    let service = labels.get("com.docker.compose.service").unwrap();

    let loadbalancer_info = LoadBalancerFactory::create(&app_state.settings.load_balancer_type)
        .get_load_balancer_info(insights.clone());

    let mut used_registry: Option<String> = None;

    // Inspect the image and try to get the registry from the first repo tag
    if let Some(image) = insights.image {
        let image_info = app_state.docker.inspect_image(&image).await?;
        if let Some(tags) = image_info.repo_tags.filter(|t| !t.is_empty()) {
            if let Some(parts) = tags[0].split('/').next() {
                let found = app_state.settings.docker.registries.iter().find(|(_, s)| {
                    s.registry
                        .trim_start_matches("http://")
                        .trim_start_matches("https://")
                        == parts
                });

                if let Some((name, _)) = found {
                    used_registry = Some(name.to_string());
                }
            }
        }
    }

    let container_state = ContainerState {
        status: state.status.unwrap().into(),
        id: Some(container_id.to_string()),
        service: service.to_string(),
        domains: loadbalancer_info.domains.clone(),
        use_tls: loadbalancer_info.tls_enabled,
        port: loadbalancer_info.port,
        started_at: Some(local_date),
        used_registry,
        basic_auth: match (
            loadbalancer_info.basic_auth_user,
            loadbalancer_info.basic_auth_pass,
        ) {
            (Some(user), Some(pass)) => Some((user, pass)),
            _ => None,
        },
    };

    Ok(container_state)
}

pub async fn collect_environment_from_app(
    app_state: &SharedAppState,
    app: &AppData,
) -> anyhow::Result<HashMap<String, String>> {
    let mut environment = HashMap::new();

    for service in &app.services {
        let Some(container_id) = &service.id else {
            continue;
        };

        match app_state
            .docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
        {
            Ok(inspect) => {
                if let Some(env) = inspect.config.and_then(|c| c.env) {
                    for env_var in env {
                        if let Some((key, value)) = env_var.split_once('=') {
                            if !key.is_empty() {
                                environment.insert(key.to_string(), value.to_string());
                            }
                        } else {
                            tracing::warn!("Invalid environment variable format: {}", env_var);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to inspect container {}: {}", container_id, e);
            }
        }
    }
    Ok(environment)
}
