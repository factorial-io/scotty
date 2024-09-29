use std::collections::HashMap;

use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{app_state::SharedAppState, settings::AppBlueprint};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AppBlueprintList {
    pub blueprints: HashMap<String, AppBlueprint>,
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/blueprints",
    responses(
    (status = 200, response = inline(AppBlueprintList))
    )
)]
pub async fn blueprints_handler(State(state): State<SharedAppState>) -> impl IntoResponse {
    let blueprints = AppBlueprintList {
        blueprints: state.settings.apps.blueprints.clone(),
    };
    

    Json(blueprints)
}
