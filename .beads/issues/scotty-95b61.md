---
title: Implement REST API endpoints for file copy in scotty
status: open
priority: 2
issue_type: task
depends_on:
  scotty-0fb7b: parent-child
created_at: 2025-12-08T23:54:03.103017+00:00
updated_at: 2025-12-08T23:54:03.103017+00:00
---

# Description

Add HTTP endpoints for file upload/download using Docker copy API.

**Endpoints**:
- POST /api/apps/{app_name}/services/{service_name}/copy-to
- GET /api/apps/{app_name}/services/{service_name}/copy-from?path=/container/path

**Implementation** (scotty/src/api/rest/handlers/files.rs):
```rust
pub async fn copy_to_container(
    State(state): State<SharedAppState>,
    Path((app_name, service_name)): Path<(String, String)>,
    Json(request): Json<CopyToContainerRequest>,
) -> Result<Json<CopyResponse>, StatusCode> {
    // Get container ID from app data
    let container_id = get_container_id(&state, &app_name, &service_name)?;
    
    // Upload tar archive using Bollard
    let options = UploadToContainerOptions {
        path: &request.container_path,
        ..Default::default()
    };
    state.docker
        .upload_to_container(container_id, Some(options), request.tar_data.into())
        .await?;
    
    Ok(Json(CopyResponse { success: true, bytes_transferred: ... }))
}

pub async fn copy_from_container(...) -> Result<Response, StatusCode> {
    // Download from container (returns tar stream)
    let stream = state.docker
        .download_from_container(container_id, Some(options))
        .await?;
    
    // Stream response with tar data
    Ok((StatusCode::OK, [("Content-Type", "application/x-tar")], StreamBody::new(stream)).into_response())
}
```

**Authorization**: Requires Permission::Manage (higher than shell access)

**Testing**:
- Test file upload with tar archive
- Test file download returns valid tar
- Test authorization checks
- Test container/service not found

**Time estimate**: 4-5 hours
