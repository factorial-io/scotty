use tabled::{
    builder::Builder,
    settings::{object::Columns, Style, Width},
};

use crate::{api::get, context::AppContext};
use scotty_core::settings::app_blueprint::AppBlueprintList;

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
        table.modify(Columns::single(3), Width::wrap(10).keep_words(true));

        ui.success("Got blueprint list!");
        Ok(table.to_string())
    })
    .await
}
