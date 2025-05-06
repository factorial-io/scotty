#![allow(dead_code)]

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::info;

use scotty_core::notification_types::{GitlabContext, Message, NotificationImpl};
use scotty_core::settings::notification_services::GitlabSettings;

pub struct NotifyGitlab {
    context: GitlabContext,
    settings: GitlabSettings,
}

impl NotifyGitlab {
    pub fn new(settings: &GitlabSettings, context: &GitlabContext) -> Self {
        NotifyGitlab {
            settings: settings.to_owned(),
            context: context.to_owned(),
        }
    }
}

async fn add_comment_to_merge_request(
    client: &Client,
    gitlab_url: &str,
    project_id: &str,
    merge_request_iid: u64,
    comment: MergeRequestComment,
    personal_access_token: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "{gitlab_url}/api/v4/projects/{project_id}/merge_requests/{merge_request_iid}/notes",
        gitlab_url = gitlab_url,
        project_id = urlencoding::encode(project_id),
        merge_request_iid = merge_request_iid
    );
    info!("Adding comment to MR: {}", url);

    let response = client
        .post(&url)
        .header("PRIVATE-TOKEN", personal_access_token)
        .json(&comment)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await?;
        Err(anyhow::anyhow!(
            "Failed to add comment. Status: {}. Error: {}",
            status,
            error_text
        ))
    }
}

#[derive(Serialize, Debug)]
struct MergeRequestComment {
    body: String,
}

#[async_trait]
impl NotificationImpl for NotifyGitlab {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()> {
        info!(
            "Sending gitlab notification to MR {} at {}",
            &self.context.mr_id, &self.settings.host
        );
        let client = Client::new();
        let comment = MergeRequestComment {
            body: format!(
                "**{}**\n\nUrls:\n{}",
                &msg.message,
                msg.urls
                    .iter()
                    .map(|u| { format!("- [{}]({})", u, u) })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        };

        add_comment_to_merge_request(
            &client,
            &self.settings.host,
            &self.context.project_id,
            self.context.mr_id,
            comment,
            &self.settings.token,
        )
        .await
    }
}
