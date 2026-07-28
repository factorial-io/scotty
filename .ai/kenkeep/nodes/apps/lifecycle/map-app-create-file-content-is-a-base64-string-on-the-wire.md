---
type: map
title: App-create file content is a base64 string on the wire
description: >-
  File.content serializes as a base64 string; deserialization also accepts a
  legacy JSON int array and strips the extra base64 layer.
tags:
  - api
  - app-create
  - serialization
  - file-upload
kk_schema_version: 3
kk_id: map-app-create-file-content-is-a-base64-string-on-the-wire
kk_derived_from:
  - 'f2e204e5-c4ad-4433-b498-0707aeed9618:map:0'
kk_relates_to:
  - practice-wire-format-changes-must-be-backward-compatible-in-both-directions
  - map-scottyctl-cli-structure
kk_depends_on: []
kk_confidence: high
---
`FileContent` (`scotty-core/src/apps/file_list.rs`) is the content field of each `File` in an `apps/create` payload. It serializes as a **base64 JSON string** and carries gzip-compressed bytes when the sibling `compressed` flag is set.

Deserialization is a hand-written visitor that accepts two forms: a base64 string (the current encoding), and a JSON array of integers (legacy scottyctl, which base64-encoded the content into a `Vec<u8>` so serde_json emitted it as an int array). The array form sets an internal `double_encoded` marker so `FileContent::decode()` strips the extra base64 layer. Re-serializing always normalizes to the string form.

The two forms are distinguishable on the wire (JSON string vs. JSON array), so no version negotiation happens. The int-array form inflates a request to roughly 5.8x the compressed size and can trip `api.create_app_max_size`; the string form is about 1.33x.

Any non-scottyctl client driving the API directly must emit one of these two forms. When changing this encoding, verify both directions of client/server version skew — an older server rejects an unknown form with a bare 422.

<!-- kk:related:start -->
# Related

- Related: [practice-wire-format-changes-must-be-backward-compatible-in-both-directions](/cli/practice-wire-format-changes-must-be-backward-compatible-in-both-directions.md)
- Related: [map-scottyctl-cli-structure](/cli/map-scottyctl-cli-structure.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [f2e204e5-c4ad-4433-b498-0707aeed9618:map:0](f2e204e5-c4ad-4433-b498-0707aeed9618:map:0)
<!-- kk:citations:end -->
