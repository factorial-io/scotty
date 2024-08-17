#![allow(dead_code)]

use std::collections::HashMap;

use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ServicePortMapping {
    pub service: String,
    pub port: u32,
}

#[derive(Debug, PartialEq, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppTtl {
    Hours(u32),
    Days(u32),
    Forever,
}
impl From<AppTtl> for u32 {
    fn from(val: AppTtl) -> Self {
        match val {
            AppTtl::Hours(h) => h * 3600,
            AppTtl::Days(d) => d * 86400,
            AppTtl::Forever => u32::MAX,
        }
    }
}

impl From<u64> for AppTtl {
    fn from(val: u64) -> Self {
        match val {
            x if x == u64::MAX => AppTtl::Forever,
            x if x % 86400 == 0 => AppTtl::Days((x / 86400) as u32),
            x => AppTtl::Hours((x / 3600) as u32),
        }
    }
}
impl Serialize for AppTtl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            AppTtl::Hours(h) => serializer.serialize_newtype_variant("AppTtl", 0, "Hours", &h),
            AppTtl::Days(d) => serializer.serialize_newtype_variant("AppTtl", 1, "Days", &d),
            AppTtl::Forever => serializer.serialize_unit_variant("AppTtl", 2, "Forever"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppSettings {
    pub needs_setup: bool,
    pub public_services: Vec<ServicePortMapping>,
    pub domain: String,
    pub time_to_live: AppTtl,
    pub basic_auth: Option<(String, String)>,
    pub disallow_robots: bool,
    pub environment: HashMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            needs_setup: false,
            public_services: Vec::new(),
            domain: "".to_string(),
            time_to_live: AppTtl::Days(1),
            basic_auth: None,
            disallow_robots: true,
            environment: HashMap::new(),
        }
    }
}

impl AppSettings {
    pub fn merge_with_global_settings(&self, setting: &Apps, app_name: &str) -> AppSettings {
        AppSettings {
            domain: format!("{}.{}", app_name, setting.domain_suffix),
            ..self.clone()
        }
    }
}

pub use bollard::models::ContainerStateStatusEnum as ContainerStatus;

use crate::settings::Apps;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ContainerState {
    pub status: ContainerStatus,
    pub id: Option<String>,
    pub service: String,
    pub domain: Option<String>,
    pub port: Option<u32>,
    pub started_at: Option<chrono::DateTime<chrono::Local>>,
}

impl Default for ContainerState {
    fn default() -> Self {
        ContainerState {
            status: ContainerStatus::EMPTY,
            id: None,
            service: "".to_string(),
            domain: None,
            port: None,
            started_at: None,
        }
    }
}

impl ContainerState {
    pub fn get_url(&self) -> Option<String> {
        self.domain
            .as_ref()
            .map(|domain| format!("http://{}", domain))
    }

    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::RUNNING
            || self.status == ContainerStatus::CREATED
            || self.status == ContainerStatus::RESTARTING
    }

    pub fn running_since(&self) -> Option<TimeDelta> {
        self.started_at
            .map(|started_at| chrono::Local::now() - started_at)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppState {
    Stopped,
    Starting,
    Running,
    Creating,
    Destroying,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppState::Stopped => write!(f, "Stopped"),
            AppState::Starting => write!(f, "Starting"),
            AppState::Running => write!(f, "Running"),
            AppState::Creating => write!(f, "Creating"),
            AppState::Destroying => write!(f, "Destroying"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppData {
    pub status: AppState,
    pub name: String,
    pub root_directory: String,
    pub docker_compose_path: String,
    pub services: Vec<ContainerState>,
    pub settings: Option<AppSettings>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            status: AppState::Stopped,
            name: "".to_string(),
            root_directory: "".to_string(),
            docker_compose_path: "".to_string(),
            services: Vec::new(),
            settings: None,
        }
    }
}

impl AppData {
    pub fn new(
        name: &str,
        root_directory: &str,
        docker_compose_path: &str,
        services: Vec<ContainerState>,
        settings: Option<AppSettings>,
    ) -> AppData {
        AppData {
            status: get_app_status_from_services(&services),
            name: name.to_string(),
            root_directory: root_directory.to_string(),
            docker_compose_path: docker_compose_path.to_string(),
            services,
            settings,
        }
    }

    pub fn urls(&self) -> Vec<String> {
        self.services.iter().filter_map(|s| s.get_url()).collect()
    }

    pub fn get_ttl(&self) -> AppTtl {
        self.settings
            .as_ref()
            .map(|s| s.time_to_live.clone())
            .unwrap_or(AppTtl::Hours(24))
    }
    pub fn running_since(&self) -> Option<TimeDelta> {
        let mut since: Option<TimeDelta> = None;
        for service in &self.services {
            if let Some(delta) = service.running_since() {
                since = Some(delta);
            }
        }

        since
    }
}

fn count_state(services: &[ContainerState], required: ContainerStatus) -> usize {
    services
        .iter()
        .fold(0, |acc, x| if x.status == required { acc + 1 } else { acc })
}

fn get_app_status_from_services(services: &[ContainerState]) -> AppState {
    let count_running_services = count_state(services, ContainerStatus::RUNNING);
    match count_running_services {
        0 => AppState::Stopped,
        x if x == services.len() => AppState::Running,
        _ => AppState::Starting,
    }
}
