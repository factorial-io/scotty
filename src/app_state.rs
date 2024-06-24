use std::sync::Arc;

use crate::{settings::Settings, stop_flag};

pub struct AppState {
    pub settings: Settings,
    pub stop_flag: stop_flag::StopFlag,
}

pub type SharedAppState = Arc<AppState>;

impl AppState {
    pub async fn new() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;
        println!("Used settings: {:?}", &settings);

        let stop_flag = stop_flag::StopFlag::new();
        stop_flag::register_signal_handler(&stop_flag);
        Ok(Arc::new(AppState {
            settings,
            stop_flag: stop_flag.clone(),
        }))
    }
}
