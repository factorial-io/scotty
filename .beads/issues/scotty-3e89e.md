---
title: Optimize file transfer for app:create command
status: open
priority: 2
issue_type: feature
labels:
- enhancement
- performance
- scottyctl
created_at: 2025-11-04T16:04:03.485064+00:00
updated_at: 2025-11-24T20:17:25.586061+00:00
---

# Description

Reduce payload size and improve transfer efficiency for file uploads

## Current Issues
- Base64 encoding adds ~33% overhead
- No compression (files sent raw)
- All files loaded into memory at once
- JSON serialization overhead

## Phase 1: Compression ✅ COMPLETED
Implemented gzip compression before base64 encoding.

### Results
- Original: 9.17 KB
- Compressed: 3.80 KB (58% reduction)
- Final payload: 17.73 KB (with base64)
- **Net improvement: ~30-40% vs uncompressed**

### Implementation
- Added `compressed: bool` field to File struct
- Client compresses with gzip before base64 encoding
- Server decompresses after base64 decoding
- Backward compatible (defaults to false)
- Tests added and passing

## Phase 2: Multipart Upload (RECOMMENDED)
Switch from JSON to multipart/form-data for additional 25% savings.

### Benefits
- **Eliminate 33% base64 overhead** completely
- Direct binary transfer (no encoding/decoding)
- Standard HTTP format (browser-compatible)
- Streaming potential for future optimization
- Better memory efficiency

### Size Comparison
| Scenario | Original | Phase 1 | Phase 2 |
|----------|----------|---------|---------|
| Text-heavy | 100 KB | 40 KB | **30 KB** |
| Mixed content | 100 KB | 53 KB | **40 KB** |

### Implementation Plan
1. Add new endpoint: `/apps/create-multipart`
2. Keep existing JSON endpoint for backward compatibility
3. Update scottyctl to use multipart
4. Update frontend to use multipart
5. Deprecate JSON endpoint after adoption period

### Technical Changes
**Client (scottyctl):**
- Use `reqwest::multipart::Form`
- Add files as binary parts with `application/gzip` MIME type
- Send metadata as form fields

**Server (scotty):**
- Add handler with `Multipart` extractor
- Stream and decompress files as they arrive
- Parse metadata from form fields

**Frontend:**
- Use browser's native `FormData`
- Compress with `pako.gzip`
- Append as Blob with proper MIME type

### Migration Strategy
**Dual Support (Recommended):**
- Week 1-2: Implement multipart endpoint
- Week 3: Update scottyctl
- Week 4: Update frontend
- Month 2-6: Monitor adoption
- Month 7+: Deprecate JSON endpoint

## Acceptance Criteria

### Phase 1 ✅
- [x] Add compression (gzip) before base64 encoding
- [x] Add 'compressed' flag to File struct
- [x] Update client to compress files before encoding
- [x] Update server to decompress after decoding
- [x] Add tests for compressed file transfer
- [x] Measure and document size improvements (58% compression ratio)

### Phase 2 (Future)
- [ ] Implement multipart/form-data endpoint
- [ ] Add OpenAPI documentation for multipart
- [ ] Update scottyctl to use multipart
- [ ] Update frontend to use multipart
- [ ] Add multipart integration tests
- [ ] Document migration guide
- [ ] Deprecate JSON endpoint after adoption

## Technical Details
**Phase 1 Files:**
- scottyctl/src/commands/apps/management.rs:27-73
- scotty/src/api/rest/handlers/apps/create.rs:69-104
- scotty-core/src/apps/file_list.rs:3-11

**Phase 2 Will Touch:**
- scotty/src/api/rest/handlers/apps/create.rs (new multipart handler)
- scottyctl/src/commands/apps/management.rs (switch to multipart)
- frontend/src/lib/api/ (switch to FormData)
- scotty/src/api/router.rs (new endpoint)
