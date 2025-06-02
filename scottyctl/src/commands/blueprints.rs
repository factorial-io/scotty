use std::collections::HashMap;

use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Style, Width},
};

use crate::{api::get, cli::BlueprintInfoCommand, context::AppContext};
use scotty_core::settings::app_blueprint::{ActionName, AppBlueprintList};

pub async fn list_blueprints(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line("Listing blueprints...");
    ui.run(async || {
        let result = get(context.server(), "blueprints").await?;
        let blueprints: AppBlueprintList = serde_json::from_value(result)?;

        let mut builder = Builder::default();
        builder.push_record(vec!["Id", "Name", "Description", "Required Services"]);
        for blueprint in blueprints.blueprints {
            let id = blueprint.0;
            let blueprint = blueprint.1;
            builder.push_record(vec![
                &id,
                &blueprint.name,
                &blueprint.description,
                &blueprint.required_services.join(", "),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::modern_rounded());
        table.modify(Columns::single(0), Width::wrap(15).keep_words(true));
        table.modify(Columns::single(1), Width::wrap(15).keep_words(true));
        table.modify(Columns::single(2), Width::wrap(25).keep_words(true));
        table.modify(Columns::single(3), Width::wrap(40).keep_words(true));

        ui.success("Got blueprint list!");
        Ok(table.to_string())
    })
    .await
}

fn format_services_command(commands: &HashMap<String, Vec<String>>) -> String {
    let mut services_commands = String::new();
    for (i, (service, commands)) in commands.iter().enumerate() {
        if i > 0 {
            services_commands.push_str("\n\n");
        }
        services_commands.push_str(&format!("{}:", service.blue().bold()));

        for cmd in commands {
            services_commands.push_str(&format!("\n  ▹ {}", cmd));
        }
    }
    services_commands
}

pub async fn blueprint_info(
    context: &AppContext,
    cmd: &BlueprintInfoCommand,
) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting info for blueprint {}...",
        cmd.blueprint_name.yellow()
    ));
    ui.run(async || {
        let result = get(context.server(), "blueprints").await?;
        let blueprints: AppBlueprintList = serde_json::from_value(result)?;

        // Find the specified blueprint
        let blueprint = blueprints
            .blueprints
            .get(&cmd.blueprint_name)
            .ok_or_else(|| {
                anyhow::anyhow!("Blueprint {} not found", cmd.blueprint_name.yellow())
            })?;

        // Format and display the blueprint info
        let mut output = String::new();
        output.push_str(&format!("Blueprint: {}\n", cmd.blueprint_name.yellow()));
        output.push_str(&format!("Name: {}\n", blueprint.name));
        output.push_str(&format!("Description: {}\n", blueprint.description));
        output.push_str(&format!(
            "Required Services: {}\n",
            blueprint.required_services.join(", ")
        ));

        // Display public services if any
        if let Some(public_services) = &blueprint.public_services {
            output.push_str("\nPublic Services:\n");
            let mut builder = Builder::default();
            builder.push_record(vec!["Service", "Port"]);
            for (service, port) in public_services {
                builder.push_record(vec![service, &port.to_string()]);
            }
            let table = builder.build().with(Style::modern_rounded()).to_string();
            output.push_str(&format!("{}\n", table));
        }

        // Display actions
        output.push_str("\nActions:\n");

        // First standard lifecycle actions
        let lifecycle_actions = vec![
            ActionName::PostCreate,
            ActionName::PostRun,
            ActionName::PostRebuild,
        ];

        let mut builder = Builder::default();
        builder.push_record(vec!["Action", "Type", "Description", "Services & Commands"]);

        // Configure table settings for better readability
        use tabled::settings::Padding;

        // Add lifecycle actions
        for action in lifecycle_actions {
            let action_obj = blueprint.actions.get(&action);
            if action_obj.is_none() {
                continue;
            }
            let action_obj = action_obj.unwrap();
            let action_name: String = action.clone().into();
            let action_type: String = action.clone().get_type().into();
            let description = match &action_obj.description {
                desc if desc.is_empty() => action_type.as_str(),
                desc => desc.as_str(),
            };

            // Format the services and commands in a readable wabacon
            let services_commands = format_services_command(&action_obj.commands);

            builder.push_record(vec![
                &action_name.dimmed().to_string(),
                &action_type,
                description,
                &services_commands,
            ]);
        }

        // Add custom actions
        for (action, action_obj) in &blueprint.actions {
            match action {
                ActionName::Custom(name) => {
                    let description = action_obj.description.as_str();
                    let services_commands = format_services_command(&action_obj.commands);

                    builder.push_record(vec![
                        &name.green().to_string(),
                        action.get_type().as_str(),
                        description,
                        &services_commands,
                    ]);
                }
                _ => continue, // Skip lifecycle actions already handled
            }
        }

        let mut table = builder.build();
        table
            .with(Style::modern_rounded())
            .with(Padding::new(1, 1, 0, 0));

        output.push_str(&format!("{}\n", table));

        // Add a note about how to run custom actions
        output.push_str(&format!(
            "\n{}: Custom actions can be executed with '{}'\n",
            "Note".yellow().bold(),
            "scottyctl app:action <app-name> <action-name>".green()
        ));

        ui.success(format!(
            "Info for blueprint {} retrieved successfully",
            cmd.blueprint_name.yellow()
        ));

        Ok(output)
    })
    .await
}
