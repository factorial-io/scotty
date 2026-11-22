//! Turns a create request for a templated blueprint into a plain `FileList`
//! plus environment, so the normal create pipeline can run (design D3–D5).

use globset::GlobSetBuilder;
use serde_json::{Map, Value};

use crate::api::error::AppError;
use scotty_core::apps::app_data::AppSettings;
use scotty_core::apps::file_list::{File, FileList};
use scotty_core::settings::app_blueprint::BlueprintTemplate;
use scotty_core::utils::compose::is_valid_config_file;

const GENERATE: &str = "x-scotty-generate";
const FROM: &str = "x-scotty-from";
const IMAGE: &str = "IMAGE";

fn properties(t: &BlueprintTemplate) -> Option<&Map<String, Value>> {
    t.parameters.as_ref()?.get("properties")?.as_object()
}

/// Properties Scotty fills itself (generated or derived).
fn scotty_filled(t: &BlueprintTemplate) -> Vec<String> {
    properties(t)
        .map(|p| {
            p.iter()
                .filter(|(_, v)| v.get(GENERATE).is_some() || v.get(FROM).is_some())
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// D3: rejects caller-set Scotty-filled keys and unknown keys, then validates
/// against the schema (with Scotty-filled keys dropped from `required`).
/// Also rejects caller environment entries that would shadow template keys.
pub fn validate_parameters(
    t: &BlueprintTemplate,
    params: &Map<String, Value>,
    settings: &AppSettings,
) -> Result<(), AppError> {
    let filled = scotty_filled(t);
    let known = properties(t);
    let mut errors = vec![];
    for key in params.keys() {
        if filled.contains(key) {
            errors.push(format!("{key}: set by Scotty, must not be supplied"));
        } else if !known.is_some_and(|p| p.contains_key(key)) {
            errors.push(format!("{key}: unknown parameter"));
        }
    }
    for (key, _) in settings.environment.iter() {
        if key == IMAGE || known.is_some_and(|p| p.contains_key(key)) {
            errors.push(format!(
                "{key}: is a blueprint parameter, not an environment variable"
            ));
        }
    }

    if let Some(schema) = &t.parameters {
        let mut schema = schema.clone();
        if let Some(Value::Array(req)) = schema.get_mut("required") {
            req.retain(|r| !r.as_str().is_some_and(|r| filled.iter().any(|f| f == r)));
        }
        let validator = jsonschema::validator_for(&schema)
            .map_err(|e| AppError::BadRequest(format!("invalid blueprint schema: {e}")))?;
        let instance = Value::Object(params.clone());
        for e in validator.iter_errors(&instance) {
            let path = e.instance_path().to_string();
            errors.push(if path.is_empty() {
                e.to_string()
            } else {
                format!("{}: {e}", path.trim_start_matches('/'))
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "invalid blueprint parameters: {}",
            errors.join("; ")
        )))
    }
}

/// D4: writes validated, generated and derived parameters plus `IMAGE` into
/// `settings.environment` and records generated keys. Generated values are
/// only created when absent, so re-running keeps them stable.
pub fn resolve_environment(
    t: &BlueprintTemplate,
    params: &Map<String, Value>,
    image: Option<&str>,
    app_name: &str,
    settings: &mut AppSettings,
) -> Result<(), AppError> {
    for (key, value) in params {
        let value = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        settings.environment.insert(key.clone(), value);
    }

    match (image, t.images.first()) {
        (Some(img), _) if !t.images.iter().any(|i| i == img) => {
            return Err(AppError::BadRequest(format!(
                "image {img} is not allowed by this blueprint"
            )))
        }
        (Some(img), _) => settings.environment.insert(IMAGE.into(), img.into()),
        (None, Some(default)) => settings.environment.insert(IMAGE.into(), default.clone()),
        (None, None) => {}
    }

    // The hosts the app is actually served at: `<service>.<domain>` or the
    // custom domains, public services in blueprint order.
    let mut hosts: Vec<String> = settings
        .public_services
        .iter()
        .flat_map(|s| s.get_domains(&settings.domain))
        .collect();
    if hosts.is_empty() {
        hosts.push(settings.domain.clone());
    }

    for (key, prop) in properties(t).into_iter().flatten() {
        if let Some(kind) = prop.get(GENERATE).and_then(Value::as_str) {
            if settings.environment.get(key).is_none() {
                settings.environment.insert(key.clone(), generate(kind)?);
            }
            if !settings.generated_parameters.contains(key) {
                settings.generated_parameters.push(key.clone());
            }
        } else if let Some(from) = prop.get(FROM).and_then(Value::as_str) {
            let value = match from {
                "app.domain" => hosts.first().cloned().unwrap_or_default(),
                // Anchored, escaped regexes for apps that take host patterns
                // (e.g. Drupal's trusted_host_patterns, matched unanchored).
                "app.domain_pattern" => hosts
                    .iter()
                    .map(|h| format!("^{}$", regex::escape(h)))
                    .collect::<Vec<_>>()
                    .join(","),
                "app.name" => app_name.to_string(),
                other => {
                    return Err(AppError::BadRequest(format!(
                        "{key}: unsupported {FROM} value {other}"
                    )))
                }
            };
            settings.environment.insert(key.clone(), value);
        }
    }
    Ok(())
}

/// Copies the values Scotty generated for an existing app into `settings`,
/// so `resolve_environment` doesn't generate them again (create-existing-app D4).
fn keep_generated(
    t: &BlueprintTemplate,
    existing: Option<&AppSettings>,
    settings: &mut AppSettings,
) {
    // Only for the same blueprint: another blueprint's secrets don't belong
    // in this app's environment.
    let Some(old) = existing.filter(|old| old.app_blueprint == settings.app_blueprint) else {
        return;
    };
    // Only keys the blueprint still generates: a dropped one is gone.
    let still_generated = |key: &String| {
        properties(t)
            .and_then(|p| p.get(key))
            .is_some_and(|p| p.get(GENERATE).is_some())
    };
    for key in old
        .generated_parameters
        .iter()
        .filter(|k| still_generated(k))
    {
        if let Some(value) = old.environment.get(key) {
            settings
                .environment
                .insert(key.clone(), value.expose_secret().to_string());
        }
    }
}

/// Supported `x-scotty-generate` and `x-scotty-from` values.
const GENERATE_KINDS: &[&str] = &["hex32", "hex64", "password"];
const FROM_SOURCES: &[&str] = &["app.name", "app.domain", "app.domain_pattern"];

/// Checks a templated blueprint when Scotty starts, so a broken blueprint
/// fails the server start instead of every create request with a 400: the
/// schema compiles, Scotty-filled values are supported, payload globs parse.
pub fn check_template(t: &BlueprintTemplate) -> Result<(), String> {
    if let Some(schema) = &t.parameters {
        jsonschema::validator_for(schema).map_err(|e| format!("invalid parameter schema: {e}"))?;
    }
    if let Some(Value::Array(required)) = t.parameters.as_ref().and_then(|s| s.get("required")) {
        for r in required {
            if !r
                .as_str()
                .is_some_and(|r| properties(t).is_some_and(|p| p.contains_key(r)))
            {
                return Err(format!("required {r} is not a declared property"));
            }
        }
    }
    for (key, prop) in properties(t).into_iter().flatten() {
        // Parameters end up as environment variables, so only scalars.
        if !prop
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|ty| ["string", "number", "integer", "boolean"].contains(&ty))
        {
            // Also rejects `enum`/`oneOf`/`anyOf` without a `type`.
            return Err(format!(
                "{key}: needs a `type` of string, number, integer or boolean \
                 (parameters become environment variables)"
            ));
        }
        for (keyword, allowed) in [(GENERATE, GENERATE_KINDS), (FROM, FROM_SOURCES)] {
            if let Some(v) = prop.get(keyword) {
                if !v.as_str().is_some_and(|v| allowed.contains(&v)) {
                    return Err(format!("{key}: unsupported {keyword} value {v}"));
                }
            }
        }
    }
    for pattern in &t.payload {
        payload_glob(pattern).map_err(|e| format!("invalid payload pattern {pattern}: {e}"))?;
    }
    if t.payload.is_empty() {
        if let Some(entry) = t.payload_required.first() {
            return Err(format!(
                "required payload entry {entry} but the blueprint accepts no payload"
            ));
        }
    }
    for entry in &t.payload_required {
        let glob = payload_glob(entry)
            .map_err(|e| format!("invalid required payload entry {entry}: {e}"))?;
        // A wildcard entry is checked with one path it matches; if the sample
        // misses the entry's own glob, skip rather than refuse the blueprint.
        if entry.contains(['*', '?', '[', '{']) {
            let sample = sample_path(entry);
            if !glob.compile_matcher().is_match(&sample) {
                tracing::warn!(
                    "required payload entry {entry} is not checked against the payload patterns"
                );
            } else if !t
                .payload
                .iter()
                .any(|p| payload_glob(p).is_ok_and(|g| g.compile_matcher().is_match(&sample)))
            {
                return Err(format!(
                    "required payload entry {entry} matches no path the payload patterns accept"
                ));
            }
            continue;
        }
        let fits = t
            .payload
            .iter()
            .any(|p| payload_glob(p).is_ok_and(|g| g.compile_matcher().is_match(entry)));
        if !fits {
            return Err(format!(
                "required payload entry {entry} matches none of the payload patterns"
            ));
        }
        // merge_files rejects these from callers, so no request could meet the entry.
        if is_compose_file(entry)
            || is_env_file(entry)
            || t.files.iter().any(|f| f.name.eq_ignore_ascii_case(entry))
        {
            return Err(format!(
                "required payload entry {entry} is a file callers may not send"
            ));
        }
    }
    Ok(())
}

fn generate(kind: &str) -> Result<String, AppError> {
    let random = |n: usize| -> Result<Vec<u8>, AppError> {
        let mut buf = vec![0u8; n];
        getrandom::fill(&mut buf)
            .map_err(|e| AppError::InternalServerError(format!("no OS randomness: {e}")))?;
        Ok(buf)
    };
    Ok(match kind {
        "hex32" => hex(&random(32)?),
        "hex64" => hex(&random(64)?),
        // 64-symbol alphabet, so `byte % 64` has no modulo bias.
        "password" => {
            const ALPHABET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            random(32)?
                .iter()
                .map(|b| ALPHABET[(*b % 64) as usize] as char)
                .collect()
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "unsupported {GENERATE} value {other}"
            )))
        }
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// `value` with the keys of every object in sorted order, so its JSON text
/// doesn't depend on map ordering (e.g. `serde_json`'s `preserve_order`
/// feature, which feature unification can turn on).
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, canonical(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonical).collect()),
        other => other,
    }
}

/// A payload pattern. `*` and `?` stay within one path segment, so `html/*`
/// matches `html/index.html` but not `html/a/b.html`; `**` crosses segments.
fn payload_glob(pattern: &str) -> Result<globset::Glob, globset::Error> {
    globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
}

/// One path a glob plausibly matches: `*`, `**` and `?` become `x`, a class
/// its first allowed character, `{a,b}` its first alternative. Callers check
/// the result against the glob, so a wrong guess is harmless.
fn sample_path(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                while chars.get(i + 1) == Some(&'*') {
                    i += 1;
                }
                out.push('x');
            }
            '?' => out.push('x'),
            '\\' if i + 1 < chars.len() => {
                i += 1;
                out.push(chars[i]);
            }
            '[' => {
                let negated = matches!(chars.get(i + 1), Some('!' | '^'));
                let start = i + 1 + usize::from(negated);
                // A `]` right after the opening is a member, not the end.
                let end = (start + 1..chars.len())
                    .find(|&j| chars[j] == ']')
                    .unwrap_or(chars.len());
                let class = &chars[start.min(end)..end];
                out.push(if negated {
                    ['x', 'y', 'z', '_']
                        .into_iter()
                        .find(|c| !class.contains(c))
                        .unwrap_or('x')
                } else {
                    class.first().copied().unwrap_or('x')
                });
                i = end;
            }
            '{' => {
                let mut depth = 0;
                let mut first_end = None;
                let mut j = i;
                while j < chars.len() {
                    match chars[j] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        ',' if depth == 1 && first_end.is_none() => first_end = Some(j),
                        _ => {}
                    }
                    j += 1;
                }
                let alt: String = chars[i + 1..first_end.unwrap_or(j).min(chars.len())]
                    .iter()
                    .collect();
                out.push_str(&sample_path(&alt));
                i = j;
            }
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// Paths of the files under `dir`, relative to it with `/` separators.
/// Unreadable entries are left out. Symlinks are neither followed nor listed
/// (`WalkDir` default, `is_file()` is false for them), so a link can never meet
/// a required payload entry; keep it that way.
pub fn relative_files(dir: &std::path::Path) -> Vec<String> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(dir).ok()?;
            let parts: Option<Vec<&str>> =
                rel.components().map(|c| c.as_os_str().to_str()).collect();
            Some(parts?.join("/"))
        })
        .collect()
}

/// Content hash of a templated blueprint: its contract plus all its files.
pub fn content_hash(t: &BlueprintTemplate) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let contract = canonical(serde_json::to_value(t).expect("template serializes"));
    h.update(serde_json::to_vec(&contract).expect("template serializes"));
    let mut files: Vec<_> = t.files.iter().collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    for f in files {
        h.update(f.name.as_bytes());
        h.update([0]);
        h.update((f.content.len() as u64).to_le_bytes());
        h.update(f.content.as_ref());
    }
    hex(&h.finalize())
}

/// What a caller passes for a templated blueprint, plus the app being
/// re-created, if any.
#[derive(Debug, Default, Clone, Copy)]
pub struct TemplateInputs<'a> {
    /// Values for the blueprint's parameters; `None` means none.
    pub parameters: Option<&'a Map<String, Value>>,
    /// The chosen image; `None` means the blueprint's default.
    pub image: Option<&'a str>,
    /// Settings of the app being re-created: its generated values are kept
    /// instead of generated again.
    pub existing: Option<&'a AppSettings>,
}

/// Prepares settings and files for a create request (D3–D6). For a
/// validation-only blueprint, or none, template inputs must be absent.
pub fn apply(
    blueprint: Option<&scotty_core::settings::app_blueprint::AppBlueprint>,
    inputs: &TemplateInputs<'_>,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
    existing_files: &[String],
) -> Result<(AppSettings, FileList), AppError> {
    let empty = Map::new();
    let params = inputs.parameters.unwrap_or(&empty);
    let Some(t) = blueprint.and_then(|b| b.template.as_ref()) else {
        if !params.is_empty() || inputs.image.is_some() {
            return Err(AppError::BadRequest(
                "blueprint parameters and image need a templated blueprint".into(),
            ));
        }
        return Ok((settings.clone(), files.clone()));
    };
    validate_parameters(t, params, settings)?;
    let merged = merge_files(t, files, existing_files)?;
    let mut settings = settings.clone();
    keep_generated(t, inputs.existing, &mut settings);
    resolve_environment(t, params, inputs.image, app_name, &mut settings)?;
    settings.blueprint_hash = Some(content_hash(t));
    Ok((settings, merged))
}

/// A compose file or a variant such as `docker-compose.override.yml`, which
/// Compose merges or Scotty writes itself, in any directory. Used both by
/// the `create:<blueprint>` check and by `merge_files`.
pub fn is_compose_file(name: &str) -> bool {
    is_valid_config_file(name) || is_compose_variant(std::path::Path::new(name))
}

/// A `.env` file or variant such as `.env.production`, in any directory.
/// Scotty writes the app's environment itself, so callers may not ship one.
pub fn is_env_file(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
    base == ".env" || base.starts_with(".env.")
}

fn is_compose_variant(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    (name.starts_with("compose.") || name.starts_with("docker-compose."))
        && (name.ends_with(".yml") || name.ends_with(".yaml"))
}

/// D5: caller files may only land on payload paths; blueprint files are added
/// and may not be replaced. `existing` lists the files already in the app's
/// directory on an update (empty for a new app); they count only towards the
/// required entries.
pub fn merge_files(
    t: &BlueprintTemplate,
    caller: &FileList,
    existing: &[String],
) -> Result<FileList, AppError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &t.payload {
        builder.add(
            payload_glob(pattern)
                .map_err(|e| AppError::BadRequest(format!("invalid payload pattern: {e}")))?,
        );
    }
    let payload = builder
        .build()
        .map_err(|e| AppError::BadRequest(format!("invalid payload patterns: {e}")))?;

    let mut files = t.files.clone();
    let mut met = vec![false; t.payload_required.len()];
    let required = t
        .payload_required
        .iter()
        .map(|r| {
            payload_glob(r)
                .map(|g| g.compile_matcher())
                .map_err(|e| AppError::BadRequest(format!("invalid required payload entry: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Only files a caller could have uploaded count, not `.scotty.yml` or compose files.
    // Globs match case-sensitively (`HTML/index.html` does not meet `html/**`);
    // only the compose and env checks ignore case.
    let uploadable = existing
        .iter()
        .filter(|n| payload.is_match(n.as_str()) && !is_compose_file(n) && !is_env_file(n));
    for name in uploadable {
        for (m, r) in met.iter_mut().zip(&required) {
            *m |= r.is_match(name);
        }
    }
    for f in &caller.files {
        let name = f.name.trim_start_matches("./");
        let path = std::path::Path::new(name);
        let reason = if path.is_absolute() || name.starts_with('/') {
            Some("absolute path")
        } else if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            Some("path contains ..")
        } else if is_compose_file(name) {
            Some("compose files are supplied by the blueprint")
        } else if is_env_file(name) {
            Some(".env files are managed by Scotty (exclude them with .scottyignore)")
        } else if t.files.iter().any(|b| b.name.eq_ignore_ascii_case(name)) {
            Some("would replace a blueprint file")
        } else if files[t.files.len()..]
            .iter()
            .any(|b| b.name.eq_ignore_ascii_case(name))
        {
            Some("duplicate file name")
        } else if !payload.is_match(name) {
            Some("outside the blueprint's payload paths")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(AppError::BadRequest(format!("file {}: {reason}", f.name)));
        }
        for (m, r) in met.iter_mut().zip(&required) {
            *m |= r.is_match(name);
        }
        files.push(File {
            name: name.to_string(),
            ..f.clone()
        });
    }
    let unmet: Vec<_> = t
        .payload_required
        .iter()
        .zip(&met)
        .filter(|(_, m)| !**m)
        .map(|(r, _)| r.as_str())
        .collect();
    if !unmet.is_empty() {
        return Err(AppError::BadRequest(format!(
            "missing required files: {}",
            unmet.join(", ")
        )));
    }
    Ok(FileList { files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scotty_core::apps::app_data::ServicePortMapping;
    use scotty_core::apps::file_list::FileContent;
    use serde_json::json;

    fn template(schema: Value) -> BlueprintTemplate {
        BlueprintTemplate {
            files: vec![file("compose.yml"), file("sws.toml")],
            parameters: Some(schema),
            images: vec!["img:1".into(), "img:2".into()],
            payload: vec!["html/**".into()],
            payload_required: vec![],
            domain_suffix: None,
        }
    }

    fn file(name: &str) -> File {
        File {
            name: name.into(),
            content: FileContent::new(b"x".to_vec()),
            compressed: false,
        }
    }

    fn atelier() -> BlueprintTemplate {
        template(json!({
            "type": "object",
            "required": ["SITE_NAME", "HASH_SALT"],
            "properties": {
                "SITE_NAME": {"type": "string", "minLength": 1},
                "HASH_SALT": {"type": "string", "x-scotty-generate": "hex32"},
                "TRUSTED": {"type": "string", "x-scotty-from": "app.domain"},
                "TRUSTED_RE": {"type": "string", "x-scotty-from": "app.domain_pattern"},
            }
        }))
    }

    fn params(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn err(r: Result<impl std::fmt::Debug, AppError>) -> String {
        r.unwrap_err().to_string()
    }

    // 2.2
    #[test]
    fn valid_parameters_pass() {
        let p = params(json!({"SITE_NAME": "Demo"}));
        validate_parameters(&atelier(), &p, &AppSettings::default()).unwrap();
    }

    #[test]
    fn missing_required_value_is_named() {
        let e = err(validate_parameters(
            &atelier(),
            &Map::new(),
            &AppSettings::default(),
        ));
        assert!(e.contains("SITE_NAME") && !e.contains("HASH_SALT"), "{e}");
    }

    #[test]
    fn wrong_type_is_rejected() {
        let p = params(json!({"SITE_NAME": 5}));
        let e = err(validate_parameters(&atelier(), &p, &AppSettings::default()));
        assert!(e.contains("SITE_NAME"), "{e}");
    }

    #[test]
    fn caller_set_generated_key_is_rejected() {
        let p = params(json!({"SITE_NAME": "x", "HASH_SALT": "mine"}));
        let e = err(validate_parameters(&atelier(), &p, &AppSettings::default()));
        assert!(e.contains("HASH_SALT"), "{e}");
    }

    #[test]
    fn caller_env_cannot_shadow_template_keys() {
        let mut s = AppSettings::default();
        s.environment.insert("HASH_SALT".into(), "mine".into());
        s.environment.insert("IMAGE".into(), "evil".into());
        let p = params(json!({"SITE_NAME": "x"}));
        let e = err(validate_parameters(&atelier(), &p, &s));
        assert!(e.contains("HASH_SALT") && e.contains("IMAGE"), "{e}");
    }

    #[test]
    fn re_create_keeps_generated_values() {
        let p = params(json!({"SITE_NAME": "Demo"}));
        let bp = scotty_core::settings::app_blueprint::AppBlueprint {
            name: "a".into(),
            description: String::new(),
            actions: Default::default(),
            required_services: vec![],
            public_services: None,
            template: Some(atelier()),
            templated: true,
        };
        let (first, _) = apply(
            Some(&bp),
            &TemplateInputs {
                parameters: Some(&p),
                image: None,
                existing: None,
            },
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            &[],
        )
        .unwrap();
        let (second, _) = apply(
            Some(&bp),
            &TemplateInputs {
                parameters: Some(&p),
                image: None,
                existing: Some(&first),
            },
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            &[],
        )
        .unwrap();
        let salt = |s: &AppSettings| s.environment.expose_all()["HASH_SALT"].clone();
        assert_eq!(salt(&first), salt(&second));

        // Switching blueprints doesn't carry the old secrets over.
        let mut other = first.clone();
        other.app_blueprint = Some("something-else".into());
        let (third, _) = apply(
            Some(&bp),
            &TemplateInputs {
                parameters: Some(&p),
                image: None,
                existing: Some(&other),
            },
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            &[],
        )
        .unwrap();
        assert_ne!(salt(&first), salt(&third));

        // A key the blueprint no longer generates isn't carried over.
        let mut dropped = bp.clone();
        let mut t = atelier();
        t.parameters.as_mut().unwrap()["properties"]
            .as_object_mut()
            .unwrap()
            .remove("HASH_SALT");
        t.parameters.as_mut().unwrap()["required"] = json!(["SITE_NAME"]);
        dropped.template = Some(t);
        let (fourth, _) = apply(
            Some(&dropped),
            &TemplateInputs {
                parameters: Some(&p),
                image: None,
                existing: Some(&first),
            },
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            &[],
        )
        .unwrap();
        assert!(fourth.environment.get("HASH_SALT").is_none());
        assert!(fourth.generated_parameters.is_empty());

        // The caller still can't set it, even on a re-create.
        let mut caller = AppSettings::default();
        caller.environment.insert("HASH_SALT".into(), "mine".into());
        assert!(apply(
            Some(&bp),
            &TemplateInputs {
                parameters: Some(&p),
                image: None,
                existing: Some(&first),
            },
            "demo",
            &caller,
            &FileList { files: vec![] },
            &[],
        )
        .is_err());
    }

    #[test]
    fn broken_blueprints_fail_the_startup_check() {
        check_template(&atelier()).unwrap();
        for (schema, payload, want) in [
            (
                json!({"type": "object", "properties": {"X": {"type": 5}}}),
                vec![],
                "schema",
            ),
            (
                json!({"type": "object", "properties": {"X": {"type": "string", "x-scotty-generate": "uuid"}}}),
                vec![],
                "x-scotty-generate",
            ),
            (
                json!({"type": "object", "properties": {"X": {"type": "string", "x-scotty-from": "app.ip"}}}),
                vec![],
                "x-scotty-from",
            ),
            (
                json!({"type": "object", "properties": {"X": {"type": "object"}}}),
                vec![],
                "needs a `type`",
            ),
            (
                json!({"type": "object", "properties": {"X": {"enum": ["a"]}}}),
                vec![],
                "needs a `type`",
            ),
            (
                json!({"type": "object", "required": ["Y"], "properties": {}}),
                vec![],
                "not a declared property",
            ),
            (
                json!({"type": "object"}),
                vec!["html/[".to_string()],
                "payload",
            ),
        ] {
            let mut t = template(schema);
            t.payload = payload;
            let e = check_template(&t).unwrap_err();
            assert!(e.contains(want), "{want}: {e}");
        }
    }

    // 2.3
    #[test]
    fn resolves_generated_derived_and_image() {
        let mut s = AppSettings {
            domain: "demo.example.com".into(),
            public_services: vec![ServicePortMapping {
                service: "edge".into(),
                port: 80,
                domains: vec![],
            }],
            ..Default::default()
        };
        let p = params(json!({"SITE_NAME": "Demo"}));
        resolve_environment(&atelier(), &p, Some("img:2"), "demo", &mut s).unwrap();
        let env = s.environment.expose_all();
        assert_eq!(env["SITE_NAME"], "Demo");
        assert_eq!(env["TRUSTED"], "edge.demo.example.com");
        assert_eq!(env["TRUSTED_RE"], r"^edge\.demo\.example\.com$");

        // Custom domains are what the app is served at.
        s.public_services[0].domains = vec!["a.org".into(), "b.org".into()];
        resolve_environment(&atelier(), &p, Some("img:2"), "demo", &mut s).unwrap();
        let env = s.environment.expose_all();
        assert_eq!(env["TRUSTED"], "a.org");
        assert_eq!(env["TRUSTED_RE"], r"^a\.org$,^b\.org$");
        assert_eq!(env["IMAGE"], "img:2");
        let salt = &env["HASH_SALT"];
        assert_eq!(salt.len(), 64);
        assert!(salt.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(s.generated_parameters, ["HASH_SALT"]);

        // Not regenerated when present.
        resolve_environment(&atelier(), &p, None, "demo", &mut s).unwrap();
        assert_eq!(&s.environment.expose_all()["HASH_SALT"], salt);
        assert_eq!(s.generated_parameters, ["HASH_SALT"]);
        assert_eq!(s.environment.expose_all()["IMAGE"], "img:1");
    }

    #[test]
    fn unlisted_image_is_rejected() {
        let mut s = AppSettings::default();
        let e = err(resolve_environment(
            &atelier(),
            &Map::new(),
            Some("evil"),
            "d",
            &mut s,
        ));
        assert!(e.contains("evil"), "{e}");
    }

    // 2.4
    fn caller(names: &[&str]) -> FileList {
        FileList {
            files: names.iter().map(|n| file(n)).collect(),
        }
    }

    #[test]
    fn html_payload_is_accepted() {
        let merged = merge_files(
            &atelier(),
            &caller(&["html/index.html", "./html/img/a.png"]),
            &[],
        )
        .unwrap();
        let names: Vec<_> = merged.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "compose.yml",
                "sws.toml",
                "html/index.html",
                "html/img/a.png"
            ]
        );
    }

    #[test]
    fn rejections() {
        for (name, why) in [
            ("docker-compose.yml", "compose"),
            ("nginx.conf", "payload"),
            ("html/../x", ".."),
            ("../x", ".."),
            ("/etc/passwd", "absolute"),
            ("sws.toml", "replace"),
            ("SWS.toml", "replace"),
            ("html/docker-compose.override.yml", "compose"),
            ("html/compose.override.yaml", "compose"),
            ("./html/../../x", ".."),
            ("html/./../x", ".."),
        ] {
            let e = err(merge_files(&atelier(), &caller(&[name]), &[]));
            assert!(e.contains(why), "{name}: {e}");
        }
        let e = err(merge_files(
            &atelier(),
            &caller(&["html/a.html", "./html/A.html"]),
            &[],
        ));
        assert!(e.contains("duplicate"), "{e}");
    }

    #[test]
    fn env_files_are_rejected() {
        for name in ["html/.env", "html/.env.production", "html/sub/.ENV"] {
            let e = err(merge_files(&atelier(), &caller(&[name]), &[]));
            assert!(e.contains(".env") && e.contains(name), "{name}: {e}");
        }
        for name in ["html/env.js", "html/.envrc-docs.html"] {
            assert!(
                merge_files(&atelier(), &caller(&[name]), &[]).is_ok(),
                "{name}"
            );
        }
    }

    #[test]
    fn single_star_stays_in_one_directory() {
        let mut t = atelier();
        t.payload = vec!["html/*".into()];
        assert!(merge_files(&t, &caller(&["html/index.html"]), &[]).is_ok());
        let e = err(merge_files(&t, &caller(&["html/a/b.html"]), &[]));
        assert!(e.contains("payload"), "{e}");
        // Matching is case-sensitive, like the file system.
        assert!(merge_files(&t, &caller(&["HTML/index.html"]), &[]).is_err());
    }

    #[test]
    fn content_hash_ignores_key_order_but_not_content() {
        let a = template(json!({"type": "object", "properties": {
            "A": {"type": "string"}, "B": {"type": "string"}}}));
        let b = template(json!({"properties": {
            "B": {"type": "string"}, "A": {"type": "string"}}, "type": "object"}));
        assert_eq!(content_hash(&a), content_hash(&b));

        let mut changed = a.clone();
        changed.files = vec![file("compose.yml")];
        assert_ne!(content_hash(&a), content_hash(&changed));
        changed.files[0].content = FileContent::new(b"y".to_vec());
        let before = content_hash(&changed);
        changed.files[0].content = FileContent::new(b"z".to_vec());
        assert_ne!(before, content_hash(&changed));
    }

    #[test]
    fn app_domain_falls_back_to_the_app_domain() {
        let mut s = AppSettings {
            domain: "demo.example.com".into(),
            ..Default::default()
        };
        let p = params(json!({"SITE_NAME": "Demo"}));
        resolve_environment(&atelier(), &p, None, "demo", &mut s).unwrap();
        let env = s.environment.expose_all();
        assert_eq!(env["TRUSTED"], "demo.example.com");
        assert_eq!(env["TRUSTED_RE"], r"^demo\.example\.com$");
    }

    #[test]
    fn no_payload_patterns_accept_nothing() {
        let mut t = atelier();
        t.payload.clear();
        assert!(merge_files(&t, &caller(&["html/index.html"]), &[]).is_err());
    }

    #[test]
    fn required_index_missing() {
        let mut t = atelier();
        t.payload_required = vec!["html/index.html".into()];
        let e = err(merge_files(&t, &caller(&["html/about.html"]), &[]));
        assert!(e.contains("html/index.html"), "{e}");
        let e = err(merge_files(&t, &caller(&[]), &[]));
        assert!(e.contains("html/index.html"), "{e}");
    }

    #[test]
    fn required_path_present() {
        let mut t = atelier();
        t.payload_required = vec!["html/index.html".into()];
        merge_files(&t, &caller(&["html/index.html", "html/img/a.png"]), &[]).unwrap();
    }

    #[test]
    fn required_glob_and_all_unmet_named() {
        let mut t = atelier();
        t.payload = vec!["data/*.json".into(), "html/**".into()];
        t.payload_required = vec![
            "data/*.json".into(),
            "html/index.html".into(),
            "html/b.html".into(),
        ];
        merge_files(
            &t,
            &caller(&["data/a.json", "html/index.html", "html/b.html"]),
            &[],
        )
        .unwrap();
        let e = err(merge_files(&t, &caller(&["data/a.json"]), &[]));
        assert!(
            e.contains("html/index.html") && e.contains("html/b.html"),
            "{e}"
        );
        assert!(!e.contains("data/*.json"), "{e}");
    }

    #[test]
    fn no_requirement_accepts_no_files() {
        merge_files(&atelier(), &caller(&[]), &[]).unwrap();
    }

    #[test]
    fn required_checked_at_startup() {
        let mut t = atelier();
        t.payload_required = vec!["html/index.html".into(), "html/**".into()];
        check_template(&t).unwrap();
        t.payload_required = vec!["index.html".into()];
        let e = check_template(&t).unwrap_err();
        assert!(e.contains("index.html"), "{e}");
        t.payload_required = vec!["html/[".into()];
        assert!(check_template(&t).is_err());
        for entry in [
            "html/*.{html,htm}",
            "html/**",
            "html/[ab].html",
            "html/[!x]?/**",
        ] {
            t.payload_required = vec![entry.into()];
            check_template(&t).unwrap_or_else(|e| panic!("{entry}: {e}"));
        }
        // Wildcard entry outside the payload.
        t.payload_required = vec!["data/*.json".into()];
        let e = check_template(&t).unwrap_err();
        assert!(e.contains("data/*.json"), "{e}");
        for entry in ["html/.env", "html/docker-compose.override.yml"] {
            t.payload_required = vec![entry.into()];
            let e = check_template(&t).unwrap_err();
            assert!(e.contains(entry), "{entry}: {e}");
        }
    }

    #[test]
    fn required_without_payload_fails_at_startup() {
        let mut t = atelier();
        t.payload.clear();
        t.payload_required = vec!["html/*.html".into()];
        let e = check_template(&t).unwrap_err();
        assert!(e.contains("html/*.html"), "{e}");
    }

    #[test]
    fn payload_required_hash_and_serialization() {
        let mut t = atelier();
        let v = serde_json::to_value(&t).unwrap();
        assert!(v.get("payload_required").is_none());
        // Same hash as before the field existed: the serialized contract is unchanged.
        let mut legacy = v.clone();
        legacy.as_object_mut().unwrap().remove("payload_required");
        assert_eq!(v, legacy);
        let before = content_hash(&t);
        t.payload_required = vec!["html/index.html".into()];
        assert_eq!(
            serde_json::to_value(&t).unwrap()["payload_required"],
            json!(["html/index.html"])
        );
        assert_ne!(content_hash(&t), before);
    }

    #[test]
    fn sample_paths() {
        for (glob, want) in [
            ("html/**", "html/x"),
            ("html/**/index.html", "html/x/index.html"),
            ("data/*.json", "data/x.json"),
            ("a?c", "axc"),
            ("html/[ab].html", "html/a.html"),
            ("html/[!xy].html", "html/z.html"),
            ("html/*.{html,htm}", "html/x.html"),
            ("{a/{b,c},d}", "a/b"),
        ] {
            assert_eq!(sample_path(glob), want, "{glob}");
        }
    }

    // 9.1
    #[test]
    fn update_without_files_counts_existing_files() {
        let mut t = atelier();
        t.payload_required = vec!["html/index.html".into()];
        merge_files(&t, &caller(&[]), &["html/index.html".into()]).unwrap();
        // Existing files only count towards required entries.
        let e = err(merge_files(
            &t,
            &caller(&["docker-compose.yml"]),
            &["html/index.html".into()],
        ));
        assert!(e.contains("compose"), "{e}");
        // Non-payload files in the app directory never meet a required entry.
        t.payload_required = vec!["**".into()];
        let existing = [".scotty.yml".into(), "docker-compose.yml".into()];
        assert!(merge_files(&t, &caller(&[]), &existing).is_err());
    }

    #[test]
    fn new_app_without_files_is_rejected() {
        let mut t = atelier();
        t.payload_required = vec!["html/index.html".into()];
        let e = err(merge_files(&t, &caller(&[]), &[]));
        assert!(e.contains("html/index.html"), "{e}");
    }

    #[test]
    fn relative_files_lists_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("html/img")).unwrap();
        std::fs::write(dir.path().join("html/index.html"), "").unwrap();
        std::fs::write(dir.path().join("html/img/a.png"), "").unwrap();
        std::fs::write(dir.path().join(".scotty.yml"), "").unwrap();
        let mut files = relative_files(dir.path());
        files.sort();
        assert_eq!(files, [".scotty.yml", "html/img/a.png", "html/index.html"]);
    }

    #[cfg(unix)]
    #[test]
    fn relative_files_skips_symlinks() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("sub")).unwrap();
        std::fs::write(outside.path().join("index.html"), "").unwrap();
        std::fs::write(outside.path().join("sub/a.html"), "").unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("html")).unwrap();
        std::fs::write(dir.path().join("html/about.html"), "").unwrap();
        let link = std::os::unix::fs::symlink;
        link(
            outside.path().join("index.html"),
            dir.path().join("html/index.html"),
        )
        .unwrap();
        link(outside.path().join("sub"), dir.path().join("html/sub")).unwrap();
        assert_eq!(relative_files(dir.path()), ["html/about.html"]);
    }
}
