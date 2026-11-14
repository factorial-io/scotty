//! Folds the events of `docker compose --progress json pull` into per-image
//! transfer progress. Compose prints one JSON object per line: image events
//! (`"id": "Image <name>"`) and layer events (`"parent_id": "Image <name>"`).

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

/// State of one pull, fed line by line.
#[derive(Default)]
pub struct PullProgress {
    images: Vec<Image>,
    /// The image the last event was about, for the status line.
    last: Option<usize>,
}

impl PullProgress {
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
            return Some(error.or_else(|| Some(format!("{} {}", event.id, event.text))));
        };

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

    /// `(layers done + download share of the others) / layers known`. An
    /// image without known layers counts as one unit. Can drop when new
    /// layers appear; the task keeps its overall progress monotonic.
    pub fn fraction(&self) -> f32 {
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
        let Some(image) = self.last.map(|i| &self.images[i]) else {
            return "Pulling images".to_string();
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
