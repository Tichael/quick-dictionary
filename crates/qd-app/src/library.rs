//! The installed language packs: loading them in the background, and
//! adding or removing pack files.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use qd_core::{Dictionary, Pack, SearchResult, Suggestion};
use serde::Serialize;

pub struct Library {
    pub packs_dir: PathBuf,
    cache_dir: PathBuf,
    dictionary: Mutex<Dictionary>,
    loading: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub file: String,
    pub lang: String,
    pub lang_name: String,
    pub entries: u64,
    pub size_bytes: u64,
    pub built_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedPack {
    pub file: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub dir: PathBuf,
    pub loading: bool,
    pub packs: Vec<PackInfo>,
    pub failed: Vec<FailedPack>,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

impl Library {
    pub fn new(packs_dir: PathBuf, cache_dir: PathBuf) -> Library {
        Library {
            packs_dir,
            cache_dir,
            dictionary: Mutex::new(Dictionary::default()),
            loading: AtomicBool::new(false),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::SeqCst)
    }

    /// (Re)opens every pack in the packs folder. Opening a pack for the
    /// first time builds its search index, which takes a second or two.
    pub fn reload(&self) -> Result<(), String> {
        self.loading.store(true, Ordering::SeqCst);
        let result = std::fs::create_dir_all(&self.packs_dir)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                Dictionary::open_dir(&self.packs_dir, Some(&self.cache_dir))
                    .map_err(|e| e.to_string())
            });
        let outcome = result.map(|dictionary| {
            for (path, error) in &dictionary.failed {
                eprintln!("skipping pack {}: {error}", path.display());
            }
            *self.dictionary.lock().unwrap() = dictionary;
        });
        self.loading.store(false, Ordering::SeqCst);
        outcome
    }

    pub fn pack_count(&self) -> usize {
        self.dictionary.lock().unwrap().packs.len()
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        self.dictionary
            .lock()
            .unwrap()
            .search(query, limit)
            .map_err(|e| e.to_string())
    }

    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<Suggestion> {
        self.dictionary.lock().unwrap().complete(prefix, limit)
    }

    pub fn info(&self) -> LibraryInfo {
        let dictionary = self.dictionary.lock().unwrap();
        let packs = dictionary
            .packs
            .iter()
            .map(|pack| PackInfo {
                file: file_name(&pack.path),
                lang: pack.lang.clone(),
                lang_name: pack.lang_name.clone(),
                entries: pack.entry_count().unwrap_or(0),
                size_bytes: std::fs::metadata(&pack.path).map(|m| m.len()).unwrap_or(0),
                built_at: pack.built_at.clone(),
            })
            .collect();
        let failed = dictionary
            .failed
            .iter()
            .map(|(path, error)| FailedPack {
                file: file_name(path),
                error: error.to_string(),
            })
            .collect();
        LibraryInfo {
            dir: self.packs_dir.clone(),
            loading: self.is_loading(),
            packs,
            failed,
        }
    }

    /// Copies a pack file into the packs folder, as `<lang>.db`, replacing
    /// an installed pack of the same language.
    pub fn install(&self, source: &Path) -> Result<String, String> {
        // Opening it checks that it is a pack, and tells its language.
        let lang = Pack::open(source, None)
            .map_err(|e| format!("{} is not a usable language pack: {e}", source.display()))?
            .lang;
        let target = self.packs_dir.join(format!("{lang}.db"));
        let tmp = self.packs_dir.join(format!("{lang}.db.part"));
        std::fs::create_dir_all(&self.packs_dir).map_err(|e| e.to_string())?;
        std::fs::copy(source, &tmp).map_err(|e| format!("copying {}: {e}", source.display()))?;
        self.with_packs_closed(|| std::fs::rename(&tmp, &target))
            .map_err(|e| format!("installing {}: {e}", target.display()))?;
        Ok(lang)
    }

    /// Deletes an installed pack file.
    pub fn remove(&self, file: &str) -> Result<(), String> {
        let path = self.packs_dir.join(file);
        // Only plain file names inside the packs folder.
        if path.parent() != Some(self.packs_dir.as_path())
            || path.extension().is_none_or(|e| e != "db")
        {
            return Err(format!(
                "{file} is not a pack in {}",
                self.packs_dir.display()
            ));
        }
        self.with_packs_closed(|| std::fs::remove_file(&path))
            .map_err(|e| format!("removing {file}: {e}"))
    }

    /// Runs `change` with every pack closed (Windows cannot replace or
    /// delete open files), then reopens the packs.
    fn with_packs_closed(
        &self,
        change: impl FnOnce() -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let outcome = {
            let mut dictionary = self.dictionary.lock().unwrap();
            *dictionary = Dictionary::default();
            change()
        };
        if let Err(e) = self.reload() {
            eprintln!("reloading packs: {e}");
        }
        outcome
    }
}
