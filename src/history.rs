//! Versioned, bounded local history. Geometry is rebuilt, never serialized.
use crate::{
    expression::Equation,
    geometry::Sampling,
    scene::{Helix, Scene},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_ENTRIES: usize = 200;
const MAX_FILE_BYTES: u64 = 2_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Snapshot {
    Helix {
        helix: Helix,
    },
    Surface {
        equation: String,
        sampling: Sampling,
    },
}
impl Snapshot {
    pub fn capture(scene: &Scene) -> Self {
        match &scene.surface {
            Some(s) => Self::Surface {
                equation: s.source.clone(),
                sampling: s.sampling,
            },
            None => Self::Helix { helix: scene.helix },
        }
    }
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Helix { helix } => {
                helix.validate()?;
            }
            Self::Surface { equation, sampling } => {
                Equation::parse(equation)?;
                sampling.validate()?;
            }
        }
        Ok(())
    }
    pub fn restore(&self, scene: &mut Scene) -> Result<(), String> {
        match self {
            Self::Helix { helix } => scene.replace(*helix),
            Self::Surface { equation, sampling } => scene.plot(equation, *sampling),
        }
    }
    pub fn request(&self) -> String {
        match self {
            Self::Helix { helix } => format!(
                "create a helix with radius {} and height {} and turns {}",
                helix.radius, helix.height, helix.turns
            ),
            Self::Surface { equation, .. } => format!("plot {equation}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Success { snapshot: Snapshot },
    Error { message: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub request: String,
    pub timestamp: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct History {
    version: u32,
    pub entries: Vec<Entry>,
}
impl Default for History {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}
impl History {
    pub fn record(&mut self, request: String, outcome: Outcome) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.entries.push(Entry {
            request: truncate(&request, 1024),
            timestamp,
            outcome,
        });
        if self.entries.len() > MAX_ENTRIES {
            self.entries.drain(..self.entries.len() - MAX_ENTRIES);
        }
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("History file exceeds 2 MB.".into());
        }
        let history: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("Cannot read history JSON: {e}"))?;
        if history.version != 1 {
            return Err("Unsupported history version.".into());
        }
        if history.entries.len() > MAX_ENTRIES {
            return Err("Too many history entries.".into());
        }
        for entry in &history.entries {
            if entry.request.len() > 4096 || entry.timestamp > 253402300799 {
                return Err("Invalid history entry.".into());
            }
            match &entry.outcome {
                Outcome::Success { snapshot } => snapshot.validate()?,
                Outcome::Error { message } if message.len() > 4096 => {
                    return Err("History error text too large.".into());
                }
                _ => {}
            }
        }
        Ok(history)
    }
}
pub fn truncate(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

pub struct Store {
    pub path: Option<PathBuf>,
    pub history: History,
    pub warning: Option<String>,
    blocked: bool,
}
impl Store {
    pub fn local() -> Self {
        let path = directories::BaseDirs::new()
            .map(|d| d.data_local_dir().join("MathPlayground/history.json"));
        Self::open(path)
    }
    pub fn open(path: Option<PathBuf>) -> Self {
        let mut store = Self {
            path,
            history: History::default(),
            warning: None,
            blocked: false,
        };
        let result = store.load();
        if let Err(error) = result {
            store.warning = Some(format!(
                "{error} Using session-only history; existing file is preserved. Move/repair it and restart to enable saving."
            ));
            store.blocked = true;
        }
        store
    }
    fn load(&mut self) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("No application-data directory is available.")?;
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("Cannot open history: {e}")),
        };
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.history = History::decode(&bytes)?;
        Ok(())
    }
    pub fn record(&mut self, request: String, outcome: Outcome) {
        self.history.record(request, outcome);
        self.persist();
    }
    pub fn clear(&mut self) {
        self.history.entries.clear();
        self.persist();
    }
    fn persist(&mut self) {
        if self.blocked {
            return;
        }
        self.warning = self
            .save()
            .err()
            .map(|e| format!("History is available in this session but could not be saved: {e}"));
    }
    fn save(&self) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No history path.")?;
        let parent = path.parent().ok_or("Invalid history path.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(&self.history).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_restore_context_dependent_commands_and_sampling() {
        let mut scene = Scene::default();
        scene.request("set radius 2").unwrap();
        let snapshot = Snapshot::capture(&scene);
        scene.request("set height 8").unwrap();
        snapshot.restore(&mut scene).unwrap();
        assert_eq!((scene.helix.radius, scene.helix.height), (2.0, 4.0));
        let sampling = Sampling {
            extent: [6.0; 3],
            resolution: 12,
        };
        scene.plot("x^2+y^2+z^2=25", sampling).unwrap();
        let snapshot = Snapshot::capture(&scene);
        scene.replace(Helix::default()).unwrap();
        snapshot.restore(&mut scene).unwrap();
        assert_eq!(scene.surface.unwrap().sampling, sampling);
    }
    #[test]
    fn persistence_roundtrip_atomic_replacement_and_retention() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data/history.json");
        let mut store = Store::open(Some(path.clone()));
        store.record(
            "set radius 2".into(),
            Outcome::Success {
                snapshot: Snapshot::capture(&Scene::default()),
            },
        );
        store.record(
            "bad".into(),
            Outcome::Error {
                message: "Invalid request".into(),
            },
        );
        let loaded = Store::open(Some(path.clone()));
        assert_eq!(loaded.history.entries, store.history.entries);
        assert!(loaded.warning.is_none());
        for i in 0..205 {
            store.history.record(
                i.to_string(),
                Outcome::Error {
                    message: "bad".into(),
                },
            );
        }
        assert_eq!(store.history.entries.len(), 200);
        assert_eq!(store.history.entries[0].request, "5");
        store.clear();
        assert!(Store::open(Some(path)).history.entries.is_empty());
    }
    #[test]
    fn corrupt_future_and_unwritable_stores_do_not_break_session() {
        for bytes in [b"broken".as_slice(), b"{\"version\":2,\"entries\":[]}"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("history.json");
            fs::write(&path, bytes).unwrap();
            let mut store = Store::open(Some(path.clone()));
            assert!(store.warning.is_some());
            store.record(
                "bad".into(),
                Outcome::Error {
                    message: "error".into(),
                },
            );
            assert_eq!(fs::read(path).unwrap(), bytes);
            assert_eq!(store.history.entries.len(), 1);
        }
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        fs::write(&blocker, "blocking file").unwrap();
        let mut store = Store::open(Some(blocker.join("history.json")));
        store.record(
            "bad".into(),
            Outcome::Error {
                message: "error".into(),
            },
        );
        assert!(store.warning.is_some());
        let mut store = Store::open(None);
        store.record(
            "bad".into(),
            Outcome::Error {
                message: "error".into(),
            },
        );
        assert_eq!(store.history.entries.len(), 1);
    }
    #[test]
    fn invalid_loaded_state_is_rejected_without_meshing() {
        let mut history = History::default();
        history.record(
            "bad".into(),
            Outcome::Success {
                snapshot: Snapshot::Helix {
                    helix: Helix {
                        radius: -1.0,
                        ..Default::default()
                    },
                },
            },
        );
        assert!(History::decode(&serde_json::to_vec(&history).unwrap()).is_err());
        assert!(History::decode(&vec![b' '; MAX_FILE_BYTES as usize + 1]).is_err());
        assert_eq!(truncate("ππ", 3), "π");
    }
}
