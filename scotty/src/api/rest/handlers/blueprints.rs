use axum::{debug_handler, extract::State, response::IntoResponse, Extension, Json};
use scotty_core::settings::app_blueprint::{AppBlueprint, AppBlueprintList};
use serde_json::Value;

use crate::api::middleware::authorization::AuthorizationContext;
use crate::app_state::SharedAppState;
use crate::services::authorization::decision::{Action, Target};
use crate::services::AuthorizationService;

/// Lists the blueprints the user may create apps from. A templated
/// blueprint carries its contract (schema, images, payload); Scotty-filled
/// schema properties are marked `readOnly` so forms don't ask for them.
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/blueprints",
    responses(
    (status = 200, response = inline(AppBlueprintList)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn blueprints_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
) -> impl IntoResponse {
    let user = AuthorizationService::get_user_id_for_authorization(&auth_context.user);

    let mut blueprints = std::collections::HashMap::new();
    for (name, blueprint) in &state.settings.apps.blueprints {
        // Same rule as the create handler: `create:<name>` only covers
        // templated blueprints.
        let candidate = blueprint.templated.then_some(name.as_str());
        if state
            .auth_service
            .decide(&user, Action::create(candidate), Target::Global)
            .is_allowed()
        {
            blueprints.insert(name.clone(), for_listing(blueprint));
        }
    }

    Json(AppBlueprintList { blueprints })
}

fn for_listing(blueprint: &AppBlueprint) -> AppBlueprint {
    let mut blueprint = blueprint.clone();
    let props = blueprint
        .template
        .as_mut()
        .and_then(|t| t.parameters.as_mut())
        .and_then(|schema| schema.get_mut("properties"))
        .and_then(Value::as_object_mut);
    for prop in props.into_iter().flat_map(|p| p.values_mut()) {
        if prop.get("x-scotty-generate").is_some() || prop.get("x-scotty-from").is_some() {
            prop["readOnly"] = Value::Bool(true);
        }
    }
    blueprint
}
