use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[readonly::make]
pub struct MattermostSetting {
    host: String,
    username: String,
    token: String,
}

#[derive(Debug, Deserialize, Clone)]
#[readonly::make]
pub struct GitlabSettings {
    host: String,
    token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub enum NotificationService {
    Mattermost(MattermostSetting),
    Gitlab(GitlabSettings),
}
