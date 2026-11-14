//! Folds the events of `docker compose --progress json pull` (and `up`) into
//! per-image transfer progress. Compose prints one JSON object per line: image
//! events (`"id": "Image <name>"`), layer events (`"parent_id": "Image <name>"`)
//! and, for `up`, container events (`"id": "Container <name>"`).

use scotty_types::{TransferPhase, TransferProgress};
use serde::Deserialize;

#[derive(Deserialize)]
struct Event {
    id: String,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    text: String,
    current: Option<u64>,
    total: Option<u64>,
}

#[derive(Default)]
struct Layer {
    id: String,
    current: u64,
    total: u64,
    downloaded: bool,
    extracting: bool,
    done: bool,
}

impl Layer {
    /// Download progress of the layer, from 0 to 1.
    fn fraction(&self) -> f32 {
        if self.done || self.downloaded {
            1.0
        } else if self.total > 0 {
            (self.current as f32 / self.total as f32).min(1.0)
        } else {
            0.0
        }
    }
}

struct Image {
    name: String,
    /// Only moves forward, so the output gets one line per phase.
    phase: TransferPhase,
    layers: Vec<Layer>,
    done: bool,
}

impl Image {
    fn current_phase(&self) -> TransferPhase {
        if self.done {
            TransferPhase::Done
        } else if self.layers.iter().any(|l| l.extracting && !l.done) {
            TransferPhase::Extracting
        } else if self.layers.iter().any(|l| l.current > 0 || l.downloaded) {
            TransferPhase::Downloading
        } else {
            TransferPhase::Waiting
        }
    }

    fn transfer(&self) -> TransferProgress {
        TransferProgress {
            image: self.name.clone(),
            phase: self.phase,
            current_bytes: self.layers.iter().map(|l| l.current).sum(),
            total_bytes: self.layers.iter().map(|l| l.total).sum(),
            layers_done: self.layers.iter().filter(|l| l.done).count() as u32,
            layers_total: self.layers.len() as u32,
        }
    }
}

/// State of one pull or `up`, fed line by line.
#[derive(Default)]
pub struct PullProgress {
    images: Vec<Image>,
    /// The image the last event was about, for the status line.
    last: Option<usize>,
    /// Set for `up`: the number of the app's services (0 if unknown).
    up_services: Option<usize>,
    /// Containers seen so far, and whether each has started.
    containers: Vec<(String, bool)>,
    /// The last container event, as `"<text> <container>"`, for the status line.
    last_container: Option<String>,
}

impl PullProgress {
    /// Progress of `docker compose up` for an app with `services` services.
    pub fn up(services: usize) -> Self {
        Self {
            up_services: Some(services),
            ..Self::default()
        }
    }

    fn apply_container(&mut self, name: &str, text: &str) -> Option<String> {
        self.last_container = Some(format!("{text} {name}"));
        let index = match self.containers.iter().position(|(n, _)| n == name) {
            Some(i) => i,
            None => {
                self.containers.push((name.to_string(), false));
                self.containers.len() - 1
            }
        };
        let started = &mut self.containers[index].1;
        // `Running`: the container was already up and is left as it is.
        let now_started = matches!(text, "Started" | "Running");
        let line = (text == "Started" && !*started).then(|| format!("Started container {name}"));
        *started |= now_started;
        line
    }

    /// Apply one output line. `None` if it is no compose event (pass it
    /// through as output); otherwise a readable output line, if any.
    pub fn apply(&mut self, line: &str) -> Option<Option<String>> {
        let event: Event = serde_json::from_str(line).ok()?;
        let (image_id, layer_id) = match &event.parent_id {
            Some(parent) => (parent.as_str(), Some(event.id.as_str())),
            None => (event.id.as_str(), None),
        };
        let error = (event.status == "Error").then(|| format!("{}: {}", event.id, event.text));
        let Some(name) = image_id.strip_prefix("Image ") else {
            if let Some(container) = event.id.strip_prefix("Container ") {
                let line = self.apply_container(container, &event.text);
                return Some(error.or(line));
            }
            return Some(error.or_else(|| Some(format!("{} {}", event.id, event.text))));
        };
        self.last_container = None;

        let (index, mut line) = match self.images.iter().position(|i| i.name == name) {
            Some(index) => (index, None),
            None => {
                self.images.push(Image {
                    name: name.to_string(),
                    phase: TransferPhase::Waiting,
                    layers: Vec::new(),
                    done: false,
                });
                (self.images.len() - 1, Some(format!("Pulling image {name}")))
            }
        };
        self.last = Some(index);
        let image = &mut self.images[index];
        match layer_id {
            None => image.done |= event.status == "Done",
            Some(id) => {
                let layer = match image.layers.iter().position(|l| l.id == id) {
                    Some(i) => &mut image.layers[i],
                    None => {
                        image.layers.push(Layer {
                            id: id.to_string(),
                            ..Layer::default()
                        });
                        image.layers.last_mut().expect("just pushed")
                    }
                };
                match event.text.as_str() {
                    "Already exists" | "Pull complete" => {
                        layer.done = true;
                        layer.downloaded = true;
                    }
                    "Downloading" => {
                        layer.current = event.current.unwrap_or(layer.current);
                        layer.total = event.total.unwrap_or(layer.total);
                    }
                    "Verifying Checksum" | "Download complete" => {
                        layer.downloaded = true;
                        layer.current = layer.total;
                    }
                    "Extracting" => {
                        layer.downloaded = true;
                        layer.extracting = true;
                        layer.current = layer.total;
                    }
                    _ => {}
                }
            }
        }

        let phase = image.current_phase();
        if phase > image.phase {
            image.phase = phase;
            line = Some(match phase {
                TransferPhase::Waiting => format!("Pulling image {name}"),
                TransferPhase::Downloading => format!("Downloading image {name}"),
                TransferPhase::Extracting => format!("Extracting image {name}"),
                TransferPhase::Done => format!("Pulled image {name}"),
            });
        }
        Some(error.or(line))
    }

    pub fn transfers(&self) -> Vec<TransferProgress> {
        self.images.iter().map(Image::transfer).collect()
    }

    /// For a pull, the pull's share (see [`Self::pull_fraction`]). For `up`,
    /// started containers out of the app's services (or of the containers
    /// seen, if the count is unknown), with the pull weighing 80 % if `up`
    /// pulled anything.
    pub fn fraction(&self) -> f32 {
        let Some(services) = self.up_services else {
            return self.pull_fraction();
        };
        let total = if services > 0 {
            services
        } else {
            self.containers.len()
        };
        let started = self.containers.iter().filter(|(_, s)| *s).count();
        let started = if total == 0 {
            0.0
        } else {
            (started as f32 / total as f32).min(1.0)
        };
        if self.images.is_empty() {
            started
        } else {
            0.8 * self.pull_fraction() + 0.2 * started
        }
    }

    /// `(layers done + download share of the others) / layers known`. An
    /// image without known layers counts as one unit. Can drop when new
    /// layers appear; the task keeps its overall progress monotonic.
    fn pull_fraction(&self) -> f32 {
        let (done, units) = self
            .images
            .iter()
            .fold((0.0, 0usize), |(done, units), image| {
                if image.layers.is_empty() || image.done {
                    let n = image.layers.len().max(1);
                    let d = if image.done { n as f32 } else { 0.0 };
                    (done + d, units + n)
                } else {
                    let d: f32 = image.layers.iter().map(Layer::fraction).sum();
                    (done + d, units + image.layers.len())
                }
            });
        if units == 0 {
            0.0
        } else {
            done / units as f32
        }
    }

    /// One line on the image the pull is busy with.
    pub fn status(&self) -> String {
        if let Some(container) = &self.last_container {
            return container.clone();
        }
        let Some(image) = self.last.map(|i| &self.images[i]) else {
            return if self.up_services.is_some() {
                "Starting containers".to_string()
            } else {
                "Pulling images".to_string()
            };
        };
        let t = image.transfer();
        match t.phase {
            TransferPhase::Done => format!("Pulled {}", t.image),
            TransferPhase::Extracting => format!("Extracting {}", t.image),
            _ if t.total_bytes > 0 => format!(
                "Pulling {} ({} of {})",
                t.image,
                format_bytes(t.current_bytes),
                format_bytes(t.total_bytes)
            ),
            _ => format!("Pulling {}", t.image),
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else if b >= 1e3 {
        format!("{:.0} kB", b / 1e3)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECORDED: &str = include_str!("../../tests/fixtures/compose_pull_progress.jsonl");

    fn transfer<'a>(p: &'a [TransferProgress], image: &str) -> &'a TransferProgress {
        p.iter().find(|t| t.image == image).unwrap()
    }

    /// Recorded output of `docker compose --progress json pull` for three
    /// images: one already local, one with a cached layer, one uncached.
    #[test]
    fn recorded_pull_is_folded_into_transfers() {
        let mut pull = PullProgress::default();
        let mut output = Vec::new();
        let mut fractions = Vec::new();
        let mut seen_redis_start = None;
        for line in RECORDED.lines() {
            let readable = pull.apply(line).expect("every recorded line is an event");
            output.extend(readable);
            fractions.push(pull.fraction());
            if seen_redis_start.is_none() && line.contains("Already exists") {
                seen_redis_start = Some(transfer(&pull.transfers(), "redis:7.0.15-alpine").clone());
            }
        }

        // The cached layer counts as done from the start.
        let redis = seen_redis_start.unwrap();
        assert_eq!((redis.layers_done, redis.layers_total), (1, 1));

        let transfers = pull.transfers();
        assert_eq!(transfers.len(), 3);
        let redis = transfer(&transfers, "redis:7.0.15-alpine");
        assert_eq!(redis.phase, TransferPhase::Done);
        assert_eq!((redis.layers_done, redis.layers_total), (8, 8));
        assert!(redis.current_bytes > 0);
        assert_eq!(redis.current_bytes, redis.total_bytes);
        let alpine = transfer(&transfers, "alpine:3.17");
        assert_eq!(
            (alpine.phase, alpine.layers_total),
            (TransferPhase::Done, 0)
        );
        assert_eq!(pull.fraction(), 1.0);
        assert!(fractions.iter().any(|f| *f > 0.0 && *f < 1.0));

        // Readable lines only, one per image phase change.
        assert!(output.iter().all(|l| !l.starts_with('{')), "{output:?}");
        for image in ["alpine:3.17", "busybox:1.36.0", "redis:7.0.15-alpine"] {
            let pulled = format!("Pulled image {image}");
            assert_eq!(output.iter().filter(|l| **l == pulled).count(), 1);
        }
        assert_eq!(
            output
                .iter()
                .filter(|l| l.ends_with("busybox:1.36.0"))
                .collect::<Vec<_>>(),
            [
                "Pulling image busybox:1.36.0",
                "Downloading image busybox:1.36.0",
                "Extracting image busybox:1.36.0",
                "Pulled image busybox:1.36.0",
            ]
        );
    }

    #[test]
    fn late_layer_grows_totals() {
        let mut pull = PullProgress::default();
        let ev = |id: &str, text: &str, current: u64, total: u64| {
            format!(
                r#"{{"id":"{id}","parent_id":"Image web:1","status":"Working","text":"{text}","current":{current},"total":{total}}}"#
            )
        };
        pull.apply(&ev("a", "Downloading", 50, 100)).unwrap();
        assert_eq!(pull.fraction(), 0.5);
        assert_eq!(pull.status(), "Pulling web:1 (50 B of 100 B)");
        pull.apply(&ev("b", "Downloading", 0, 300)).unwrap();
        let t = &pull.transfers()[0];
        assert_eq!(
            (t.current_bytes, t.total_bytes, t.layers_total),
            (50, 400, 2)
        );
        // The step fraction may drop; the task keeps its own monotonic.
        assert_eq!(pull.fraction(), 0.25);
    }

    /// Recorded output of `docker compose --progress json up -d` for two
    /// services whose images were not local.
    #[test]
    fn recorded_up_with_pull_counts_pull_and_started_containers() {
        let mut up = PullProgress::up(2);
        let mut output = Vec::new();
        let mut fractions = Vec::new();
        let mut statuses = Vec::new();
        for line in include_str!("../../tests/fixtures/compose_up_progress.jsonl").lines() {
            output.extend(up.apply(line).expect("every recorded line is an event"));
            fractions.push(up.fraction());
            statuses.push(up.status());
        }
        assert_eq!(up.fraction(), 1.0);
        // Pulled everything, nothing started yet: 80 %.
        let before_containers = statuses
            .iter()
            .position(|s| s.starts_with("Creating"))
            .unwrap();
        assert!((fractions[before_containers] - 0.8).abs() < 1e-6);
        assert!(fractions[..before_containers]
            .iter()
            .any(|f| *f > 0.0 && *f < 0.8));
        assert!(statuses
            .iter()
            .any(|s| s.starts_with("Pulling busybox:1.35.0 (")));
        assert!(statuses.contains(&"Starting uptest-a-1".to_string()));
        assert_eq!(up.transfers().len(), 2);

        for image in ["busybox:1.35.0", "alpine:3.16"] {
            assert!(output.contains(&format!("Pulled image {image}")));
        }
        for container in ["uptest-a-1", "uptest-b-1"] {
            let started = format!("Started container {container}");
            assert_eq!(output.iter().filter(|l| **l == started).count(), 1);
        }
        assert!(output.iter().all(|l| !l.starts_with('{')), "{output:?}");
        assert!(!output.iter().any(|l| l.contains("Creating uptest")));
    }

    #[test]
    fn up_without_pull_counts_started_containers() {
        let ev = |name: &str, text: &str| {
            format!(r#"{{"id":"Container shop-{name}-1","status":"Working","text":"{text}"}}"#)
        };
        let mut up = PullProgress::up(4);
        assert_eq!(up.status(), "Starting containers");
        for name in ["web", "db", "cache", "worker"] {
            up.apply(&ev(name, "Created")).unwrap();
        }
        assert_eq!(up.fraction(), 0.0);
        up.apply(&ev("web", "Started")).unwrap();
        up.apply(&ev("db", "Starting")).unwrap();
        assert_eq!(up.status(), "Starting shop-db-1");
        assert_eq!(
            up.apply(&ev("db", "Started")),
            Some(Some("Started container shop-db-1".to_string()))
        );
        assert_eq!(up.fraction(), 0.5);
        assert!(up.transfers().is_empty());

        // Unknown service count: the containers seen are the total.
        let mut up = PullProgress::up(0);
        up.apply(&ev("web", "Running")).unwrap();
        up.apply(&ev("db", "Created")).unwrap();
        assert_eq!(up.fraction(), 0.5);
    }

    #[test]
    fn unparseable_lines_pass_through_and_errors_are_readable() {
        let mut pull = PullProgress::default();
        assert_eq!(pull.apply("WARN[0000] some warning"), None);
        assert_eq!(pull.apply("{not json"), None);
        assert_eq!(
            pull.apply(r#"{"id":"Image nope:1","status":"Error","text":"pull access denied"}"#),
            Some(Some("Image nope:1: pull access denied".to_string()))
        );
    }
}
