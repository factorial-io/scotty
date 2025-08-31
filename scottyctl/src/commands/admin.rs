use anyhow::Context;
use owo_colors::OwoColorize;
use serde_json::json;
use tabled::{builder::Builder, settings::Style};

use scotty_core::admin::{
    CreateScopeRequest, CreateRoleRequest, 
    CreateAssignmentRequest, RemoveAssignmentRequest, 
    TestPermissionRequest, GetUserPermissionsRequest,
    SuccessResponse,
};
use crate::{
    api::{get, post, delete},
    context::AppContext,
    utils::ui::Ui,
};

/// Helper function to handle success responses from admin API calls
fn handle_success_response(
    ui: &Ui,
    result: serde_json::Value,
    success_message: String,
    error_prefix: &str,
) -> anyhow::Result<()> {
    let response: SuccessResponse = serde_json::from_value(result)?;
    if response.success {
        ui.success(success_message);
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}: {}", error_prefix, response.message))
    }
}

// Scopes Management
pub async fn list_scopes(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting list of authorization scopes from {} ...",
        context.server().server
    ));
    ui.run(async || {
        let result = get(context.server(), "admin/scopes").await?;

        let response: serde_json::Value = result;
        let scopes = response["scopes"].as_array()
            .context("Failed to parse scopes list")?;

        if scopes.is_empty() {
            return Ok("No scopes found.".to_string());
        }

        let mut builder = Builder::default();
        builder.push_record(vec!["Name", "Description", "Created At"]);
        
        for scope in scopes {
            builder.push_record(vec![
                scope["name"].as_str().unwrap_or(""),
                scope["description"].as_str().unwrap_or(""),
                scope["created_at"].as_str().unwrap_or(""),
            ]);
        }
        
        let mut table = builder.build();
        table.with(Style::rounded());
        ui.success("Scopes retrieved successfully!");
        Ok(table.to_string())
    })
    .await
}

pub async fn create_scope(context: &AppContext, cmd: &CreateScopeRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Creating scope '{}' on {} ...",
        cmd.name.bright_blue(),
        context.server().server
    ));
    
    let payload = json!({
        "name": cmd.name,
        "description": cmd.description
    });

    let result = post(context.server(), "admin/scopes", payload).await?;
    
    handle_success_response(
        ui,
        result,
        format!("Scope '{}' created successfully.", cmd.name.bright_green()),
        "Failed to create scope",
    )
}

// Roles Management
pub async fn list_roles(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting list of authorization roles from {} ...",
        context.server().server
    ));
    ui.run(async || {
        let result = get(context.server(), "admin/roles").await?;

        let response: serde_json::Value = result;
        let roles = response["roles"].as_array()
            .context("Failed to parse roles list")?;

        if roles.is_empty() {
            return Ok("No roles found.".to_string());
        }

        let mut builder = Builder::default();
        builder.push_record(vec!["Name", "Description", "Permissions"]);
        
        for role in roles {
            let permissions = role["permissions"].as_array()
                .map(|p| p.iter().map(|v| v.as_str().unwrap_or("")).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
                
            builder.push_record(vec![
                role["name"].as_str().unwrap_or(""),
                role["description"].as_str().unwrap_or(""),
                &permissions,
            ]);
        }
        
        let mut table = builder.build();
        table.with(Style::rounded());
        ui.success("Roles retrieved successfully!");
        Ok(table.to_string())
    })
    .await
}

pub async fn create_role(context: &AppContext, cmd: &CreateRoleRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Creating role '{}' on {} ...",
        cmd.name.bright_blue(),
        context.server().server
    ));
    
    let payload = json!({
        "name": cmd.name,
        "description": cmd.description,
        "permissions": cmd.permissions
    });

    let result = post(context.server(), "admin/roles", payload).await?;
    
    handle_success_response(
        ui,
        result,
        format!("Role '{}' created successfully.", cmd.name.bright_green()),
        "Failed to create role",
    )
}

// Assignments Management
pub async fn list_assignments(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting list of user assignments from {} ...",
        context.server().server
    ));
    ui.run(async || {
        let result = get(context.server(), "admin/assignments").await?;

        let response: serde_json::Value = result;
        let assignments_list = response["assignments"].as_array()
            .context("Failed to parse assignments list")?;

        if assignments_list.is_empty() {
            return Ok("No assignments found.".to_string());
        }

        let mut builder = Builder::default();
        builder.push_record(vec!["User ID", "Role", "Scopes"]);
        
        for assignment_info in assignments_list {
            let user_id = assignment_info["user_id"].as_str().unwrap_or("");
            if let Some(assignments_array) = assignment_info["assignments"].as_array() {
                for assignment in assignments_array {
                    let scopes = assignment["scopes"].as_array()
                        .map(|s| s.iter().map(|v| v.as_str().unwrap_or("")).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                        
                    builder.push_record(vec![
                        user_id,
                        assignment["role"].as_str().unwrap_or(""),
                        &scopes,
                    ]);
                }
            }
        }
        
        let mut table = builder.build();
        table.with(Style::rounded());
        ui.success("Assignments retrieved successfully!");
        Ok(table.to_string())
    })
    .await
}

pub async fn create_assignment(context: &AppContext, cmd: &CreateAssignmentRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Creating assignment for user '{}' on {} ...",
        cmd.user_id.bright_blue(),
        context.server().server
    ));
    
    let payload = json!({
        "user_id": cmd.user_id,
        "role": cmd.role,
        "scopes": cmd.scopes
    });

    let result = post(context.server(), "admin/assignments", payload).await?;
    
    handle_success_response(
        ui,
        result,
        format!("Assignment for user '{}' created successfully.", cmd.user_id.bright_green()),
        "Failed to create assignment",
    )
}

pub async fn remove_assignment(context: &AppContext, cmd: &RemoveAssignmentRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Removing assignment for user '{}' on {} ...",
        cmd.user_id.bright_blue(),
        context.server().server
    ));
    
    let payload = json!({
        "user_id": cmd.user_id,
        "role": cmd.role,
        "scopes": cmd.scopes
    });

    let result = delete(context.server(), "admin/assignments", Some(payload)).await?;
    
    handle_success_response(
        ui,
        result,
        format!("Assignment for user '{}' removed successfully.", cmd.user_id.bright_green()),
        "Failed to remove assignment",
    )
}

// Permissions Management
pub async fn list_permissions(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting list of available permissions from {} ...",
        context.server().server
    ));
    ui.run(async || {
        let result = get(context.server(), "admin/permissions").await?;

        let response: serde_json::Value = result;
        let permissions = response["permissions"].as_array()
            .context("Failed to parse permissions list")?;

        if permissions.is_empty() {
            return Ok("No permissions found.".to_string());
        }

        let mut output = String::from("Available permissions:\n");
        for permission in permissions {
            if let Some(perm_str) = permission.as_str() {
                output.push_str(&format!("  • {}\n", perm_str));
            }
        }
        ui.success("Permissions retrieved successfully!");
        Ok(output)
    })
    .await
}

pub async fn test_permission(context: &AppContext, cmd: &TestPermissionRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    let default_user = "current user".to_string();
    let user_display = cmd.user_id.as_ref().unwrap_or(&default_user);
    ui.new_status_line(format!(
        "Testing permission '{}' for user '{}' on app '{}' ...",
        cmd.permission.bright_blue(),
        user_display.bright_blue(),
        cmd.app_name.bright_blue()
    ));
    
    let payload = json!({
        "user_id": cmd.user_id,
        "app_name": cmd.app_name,
        "permission": cmd.permission
    });

    let result = post(context.server(), "admin/permissions/test", payload).await?;
    
    let allowed = result["allowed"].as_bool().unwrap_or(false);
    
    if allowed {
        ui.success(format!(
            "User '{}' {} access '{}' permission on app '{}'",
            user_display.bright_green(),
            "HAS".bright_green(),
            cmd.permission.bright_blue(),
            cmd.app_name.bright_blue()
        ));
    } else {
        ui.failed(format!(
            "User '{}' {} access '{}' permission on app '{}'",
            user_display.bright_red(),
            "DOES NOT HAVE".bright_red(),
            cmd.permission.bright_blue(),
            cmd.app_name.bright_blue()
        ));
    }
    Ok(())
}

pub async fn get_user_permissions(context: &AppContext, cmd: &GetUserPermissionsRequest) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting permissions for user '{}' from {} ...",
        cmd.user_id.bright_blue(),
        context.server().server
    ));
    ui.run(async || {
        let endpoint = format!("admin/permissions/user/{}", cmd.user_id);
        let result = get(context.server(), &endpoint).await?;

        let response: serde_json::Value = result;
        let permissions = response["permissions"].as_object()
            .context("Failed to parse user permissions")?;

        if permissions.is_empty() {
            return Ok(format!("No permissions found for user '{}'.", cmd.user_id));
        }

        let mut builder = Builder::default();
        builder.push_record(vec!["Scope", "Permissions"]);
        
        for (scope, perms) in permissions {
            let perms_str = if let Some(perms_array) = perms.as_array() {
                perms_array.iter()
                    .map(|v| v.as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                String::new()
            };
            
            builder.push_record(vec![scope, &perms_str]);
        }
        
        let mut table = builder.build();
        table.with(Style::rounded());
        ui.success(format!("Permissions for user '{}' retrieved successfully!", cmd.user_id.bright_blue()));
        Ok(format!("Permissions for user '{}':\n{}", cmd.user_id.bright_blue(), table.to_string()))
    })
    .await
}