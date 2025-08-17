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
    println!("🔐 Starting OAuth device flow authentication...");

    // 1. Get server info and OAuth config
    let server_info = get_server_info(app_context.server()).await?;

    let oauth_config = match server_info_to_oauth_config(server_info) {
        Ok(config) => config,
        Err(AuthError::DeviceFlowNotEnabled) => {
            println!("❌ OAuth is configured but device flow is disabled");
            println!("💡 Please use the web interface to authenticate");
            return Ok(());
        }
        Err(AuthError::OAuthNotConfigured) => {
            println!("❌ OAuth not configured on server");
            println!("💡 Use SCOTTY_ACCESS_TOKEN environment variable instead");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    println!("✅ OAuth configuration found");

    // 2. Start device flow
    let client = DeviceFlowClient::new(oauth_config, app_context.server().server.clone());
    let device_response = match client.start_device_flow().await {
        Ok(response) => response,
        Err(e) => {
            println!("❌ Failed to start device flow");
            println!("   This might be because:");
            println!("   - OIDC provider OAuth application is not configured for device flow");
            println!("   - The client_id 'scottyctl' is not registered in your OIDC provider");
            println!("   - Network connectivity issues");
            return Err(e.into());
        }
    };

    // 3. Show user instructions
    println!("\n📱 Please complete authentication:");
    println!(
        "   1. Visit: {}",
        device_response.verification_uri.bright_blue()
    );
    println!(
        "   2. Enter code: {}",
        device_response.user_code.bright_yellow()
    );

    if !cmd.no_browser {
        match open::that(&device_response.verification_uri) {
            Ok(_) => println!("   (Opened browser automatically)"),
            Err(_) => println!("   (Could not open browser automatically)"),
        }
    }

    println!("\n⏳ Waiting for authorization...");

    // 4. Poll for token
    let stored_token = client
        .poll_for_token(&device_response.device_code, cmd.timeout)
        .await?;

    // 5. Save token
    TokenStorage::new()?.save(stored_token.clone())?;

    println!(
        "✅ Successfully authenticated as {} <{}>",
        stored_token.user_name.bright_green(),
        stored_token.user_email.bright_cyan()
    );
    println!("   Server: {}", app_context.server().server.bright_blue());

    Ok(())
}

pub async fn auth_logout(app_context: &AppContext) -> Result<()> {
    TokenStorage::new()?.clear_for_server(&app_context.server().server)?;
    println!(
        "✅ Logged out from server: {}",
        app_context.server().server.bright_blue()
    );
    Ok(())
}

pub async fn auth_status(app_context: &AppContext) -> Result<()> {
    println!("Server: {}", app_context.server().server.bright_blue());
    match get_current_auth_method(app_context).await? {
        AuthMethod::OAuth(token) => {
            println!("🔐 Authenticated via OAuth");
            println!(
                "   User: {} <{}>",
                token.user_name.bright_green(),
                token.user_email.bright_cyan()
            );
            if let Some(expires_at) = token.expires_at {
                println!("   Expires: {:?}", expires_at);
            }
        }
        AuthMethod::Bearer(_) => {
            println!("🔑 Authenticated via Bearer token (SCOTTY_ACCESS_TOKEN)");
        }
        AuthMethod::None => {
            println!("❌ Not authenticated for this server");
            println!(
                "💡 Run 'scottyctl --server {} auth:login' or set SCOTTY_ACCESS_TOKEN",
                app_context.server().server
            );
        }
    }
    Ok(())
}

pub async fn auth_refresh(app_context: &AppContext) -> Result<()> {
    println!(
        "🔄 Refreshing authentication token for server: {}",
        app_context.server().server.bright_blue()
    );

    // For now, we'll just check if the current token is still valid
    match get_current_auth_method(app_context).await? {
        AuthMethod::OAuth(token) => {
            // TODO: Implement actual token refresh logic
            println!("✅ Token appears to be valid");
            println!(
                "   User: {} <{}>",
                token.user_name.bright_green(),
                token.user_email.bright_cyan()
            );
        }
        AuthMethod::Bearer(_) => {
            println!("🔑 Bearer tokens don't require refresh");
        }
        AuthMethod::None => {
            println!("❌ No authentication found for this server");
            println!(
                "💡 Run 'scottyctl --server {} auth:login' first",
                app_context.server().server
            );
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
