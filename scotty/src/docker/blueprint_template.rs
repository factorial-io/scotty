//! Turns a create request for a templated blueprint into a plain `FileList`
//! plus environment, so the normal create pipeline can run (design D3–D5).

use globset::{Glob, GlobSetBuilder};
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
fn keep_generated(existing: Option<&AppSettings>, settings: &mut AppSettings) {
    // Only for the same blueprint: another blueprint's secrets don't belong
    // in this app's environment.
    let Some(old) = existing.filter(|old| old.app_blueprint == settings.app_blueprint) else {
        return;
    };
    for key in &old.generated_parameters {
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
    for (key, prop) in properties(t).into_iter().flatten() {
        for (keyword, allowed) in [(GENERATE, GENERATE_KINDS), (FROM, FROM_SOURCES)] {
            if let Some(v) = prop.get(keyword) {
                if !v.as_str().is_some_and(|v| allowed.contains(&v)) {
                    return Err(format!("{key}: unsupported {keyword} value {v}"));
                }
            }
        }
    }
    for pattern in &t.payload {
        Glob::new(pattern).map_err(|e| format!("invalid payload pattern {pattern}: {e}"))?;
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
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Content hash of a templated blueprint: its contract plus all its files.
pub fn content_hash(t: &BlueprintTemplate) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(serde_json::to_vec(t).expect("template serializes"));
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

/// Prepares settings and files for a create request (D3–D6). For a
/// validation-only blueprint, or none, template inputs must be absent.
/// `existing` is the app being re-created, if any: its generated values are
/// kept instead of generated again.
pub fn apply(
    blueprint: Option<&scotty_core::settings::app_blueprint::AppBlueprint>,
    params: &Map<String, Value>,
    image: Option<&str>,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
    existing: Option<&AppSettings>,
) -> Result<(AppSettings, FileList), AppError> {
    let Some(t) = blueprint.and_then(|b| b.template.as_ref()) else {
        if !params.is_empty() || image.is_some() {
            return Err(AppError::BadRequest(
                "blueprint parameters and image need a templated blueprint".into(),
            ));
        }
        return Ok((settings.clone(), files.clone()));
    };
    validate_parameters(t, params, settings)?;
    let merged = merge_files(t, files)?;
    let mut settings = settings.clone();
    keep_generated(existing, &mut settings);
    resolve_environment(t, params, image, app_name, &mut settings)?;
    settings.blueprint_hash = Some(content_hash(t));
    Ok((settings, merged))
}

/// D5: caller files may only land on payload paths; blueprint files are added
/// and may not be replaced.
pub fn merge_files(t: &BlueprintTemplate, caller: &FileList) -> Result<FileList, AppError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &t.payload {
        builder.add(
            Glob::new(pattern)
                .map_err(|e| AppError::BadRequest(format!("invalid payload pattern: {e}")))?,
        );
    }
    let payload = builder
        .build()
        .map_err(|e| AppError::BadRequest(format!("invalid payload patterns: {e}")))?;

    let mut files = t.files.clone();
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
        } else if is_valid_config_file(name) {
            Some("compose files are supplied by the blueprint")
        } else if t.files.iter().any(|b| b.name == name) {
            Some("would replace a blueprint file")
        } else if !payload.is_match(name) {
            Some("outside the blueprint's payload paths")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(AppError::BadRequest(format!("file {}: {reason}", f.name)));
        }
        files.push(File {
            name: name.to_string(),
            ..f.clone()
        });
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
            &p,
            None,
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            None,
        )
        .unwrap();
        let (second, _) = apply(
            Some(&bp),
            &p,
            None,
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            Some(&first),
        )
        .unwrap();
        let salt = |s: &AppSettings| s.environment.expose_all()["HASH_SALT"].clone();
        assert_eq!(salt(&first), salt(&second));

        // Switching blueprints doesn't carry the old secrets over.
        let mut other = first.clone();
        other.app_blueprint = Some("something-else".into());
        let (third, _) = apply(
            Some(&bp),
            &p,
            None,
            "demo",
            &AppSettings::default(),
            &FileList { files: vec![] },
            Some(&other),
        )
        .unwrap();
        assert_ne!(salt(&first), salt(&third));

        // The caller still can't set it, even on a re-create.
        let mut caller = AppSettings::default();
        caller.environment.insert("HASH_SALT".into(), "mine".into());
        assert!(apply(
            Some(&bp),
            &p,
            None,
            "demo",
            &caller,
            &FileList { files: vec![] },
            Some(&first)
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
                json!({"type": "object", "properties": {"X": {"x-scotty-generate": "uuid"}}}),
                vec![],
                "x-scotty-generate",
            ),
            (
                json!({"type": "object", "properties": {"X": {"x-scotty-from": "app.ip"}}}),
                vec![],
                "x-scotty-from",
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
        ] {
            let e = err(merge_files(&atelier(), &caller(&[name])));
            assert!(e.contains(why), "{name}: {e}");
        }
    }

    #[test]
    fn no_payload_patterns_accept_nothing() {
        let mut t = atelier();
        t.payload.clear();
        assert!(merge_files(&t, &caller(&["html/index.html"])).is_err());
    }
}
