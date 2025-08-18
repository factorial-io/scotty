use crate::auth::{
    config::{get_server_info, server_info_to_oauth_config},
    device_flow::DeviceFlowClient,
    storage::TokenStorage,
    AuthError, AuthMethod,
};
use crate::cli::AuthLoginCommand;
use crate::context::AppContext;
use anyhow::Result;
use owo_colors::OwoColorize;

pub async fn auth_login(app_context: &AppContext, cmd: &AuthLoginCommand) -> Result<()> {
    app_context.ui().println("Starting OAuth device flow authentication...");

    // 1. Get server info and OAuth config
    let server_info = get_server_info(app_context.server()).await?;

    let oauth_config = match server_info_to_oauth_config(server_info) {
        Ok(config) => config,
        Err(AuthError::DeviceFlowNotEnabled) => {
            app_context.ui().failed("OAuth is configured but device flow is disabled");
            app_context.ui().println("Please use the web interface to authenticate");
            return Ok(());
        }
        Err(AuthError::OAuthNotConfigured) => {
            app_context.ui().failed("OAuth not configured on server");
            app_context.ui().println("Use SCOTTY_ACCESS_TOKEN environment variable instead");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    app_context.ui().success("OAuth configuration found");

    // 2. Start device flow
    let client = DeviceFlowClient::new(oauth_config, app_context.server().server.clone());
    let device_response = match client.start_device_flow().await {
        Ok(response) => response,
        Err(e) => {
            app_context.ui().failed("Failed to start device flow");
            app_context.ui().println("   This might be because:");
            app_context.ui().println("   - OIDC provider OAuth application is not configured for device flow");
            app_context.ui().println("   - The client_id 'scottyctl' is not registered in your OIDC provider");
            app_context.ui().println("   - Network connectivity issues");
            return Err(e.into());
        }
    };

    // 3. Show user instructions
    app_context.ui().println("\nPlease complete authentication:");
    app_context.ui().println(&format!(
        "   1. Visit: {}",
        device_response.verification_uri.bright_blue()
    ));
    app_context.ui().println(&format!(
        "   2. Enter code: {}",
        device_response.user_code.bright_yellow()
    ));

    if !cmd.no_browser {
        match open::that(&device_response.verification_uri) {
            Ok(_) => app_context.ui().println("   (Opened browser automatically)"),
            Err(_) => app_context.ui().println("   (Could not open browser automatically)"),
        }
    }

    app_context.ui().println("\nWaiting for authorization...");

    // 4. Poll for token
    let stored_token = client
        .poll_for_token(&device_response.device_code, cmd.timeout)
        .await?;

    // 5. Save token
    TokenStorage::new()?.save(stored_token.clone())?;

    app_context.ui().success(&format!(
        "Successfully authenticated as {} <{}>",
        stored_token.user_name.bright_green(),
        stored_token.user_email.bright_cyan()
    ));
    app_context.ui().println(&format!("   Server: {}", app_context.server().server.bright_blue()));

    Ok(())
}

pub async fn auth_logout(app_context: &AppContext) -> Result<()> {
    TokenStorage::new()?.clear_for_server(&app_context.server().server)?;
    app_context.ui().success(&format!(
        "Logged out from server: {}",
        app_context.server().server.bright_blue()
    ));
    Ok(())
}

pub async fn auth_status(app_context: &AppContext) -> Result<()> {
    app_context.ui().println(&format!("Server: {}", app_context.server().server.bright_blue()));
    match get_current_auth_method(app_context).await? {
        AuthMethod::OAuth(token) => {
            app_context.ui().println("Authenticated via OAuth");
            app_context.ui().println(&format!(
                "   User: {} <{}>",
                token.user_name.bright_green(),
                token.user_email.bright_cyan()
            ));
            if let Some(expires_at) = token.expires_at {
                app_context.ui().println(&format!("   Expires: {:?}", expires_at));
            }
        }
        AuthMethod::Bearer(_) => {
            app_context.ui().println("Authenticated via Bearer token (SCOTTY_ACCESS_TOKEN)");
        }
        AuthMethod::None => {
            app_context.ui().println("Not authenticated for this server");
            app_context.ui().println(&format!(
                "Run 'scottyctl --server {} auth:login' or set SCOTTY_ACCESS_TOKEN",
                app_context.server().server
            ));
        }
    }
    Ok(())
}

pub async fn auth_refresh(app_context: &AppContext) -> Result<()> {
    app_context.ui().println(&format!(
        "Refreshing authentication token for server: {}",
        app_context.server().server.bright_blue()
    ));

    // For now, we'll just check if the current token is still valid
    match get_current_auth_method(app_context).await? {
        AuthMethod::OAuth(token) => {
            // TODO: Implement actual token refresh logic
            app_context.ui().success("Token appears to be valid");
            app_context.ui().println(&format!(
                "   User: {} <{}>",
                token.user_name.bright_green(),
                token.user_email.bright_cyan()
            ));
        }
        AuthMethod::Bearer(_) => {
            app_context.ui().println("Bearer tokens don't require refresh");
        }
        AuthMethod::None => {
            app_context.ui().failed("No authentication found for this server");
            app_context.ui().println(&format!(
                "Run 'scottyctl --server {} auth:login' first",
                app_context.server().server
            ));
        }
    }

    Ok(())
}

async fn get_current_auth_method(app_context: &AppContext) -> Result<AuthMethod> {
    let server_url = &app_context.server().server;
    tracing::debug!("Checking auth for server: {}", server_url);

    // 1. Try stored OAuth token first
    if let Ok(Some(stored_token)) = TokenStorage::new()?.load_for_server(server_url) {
        tracing::debug!(
            "Found stored OAuth token for user: {}",
            stored_token.user_email
        );
        return Ok(AuthMethod::OAuth(stored_token));
    } else {
        tracing::debug!("No stored OAuth token found for server: {}", server_url);
    }

    // 2. Fall back to environment variable
    if let Some(token) = &app_context.server().access_token {
        tracing::debug!("Using bearer token from environment");
        return Ok(AuthMethod::Bearer(token.clone()));
    }

    tracing::debug!("No authentication method available");
    Ok(AuthMethod::None)
}
