//! Downloading published language packs (the "packs" GitHub release, see
//! .github/workflows/packs.yml and spec/pack-format.md).

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::i18n::{self, Lang};
use qd_core::SCHEMA_VERSION;

/// Where the packs are published. QD_PACKS_URL overrides it (testing).
const PACKS_URL: &str = "https://github.com/Tichael/quick-dictionary/releases/download/packs/";

const MANIFEST_VERSION: u32 = 1;

/// The address packs are downloaded from, ending with "/".
pub fn base_url() -> String {
    let url = std::env::var("QD_PACKS_URL").unwrap_or_else(|_| PACKS_URL.into());
    if url.ends_with('/') {
        url
    } else {
        format!("{url}/")
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    manifest_version: u32,
    packs: Vec<RemotePack>,
}

/// A pack listed in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct RemotePack {
    pub lang: String,
    pub lang_name: String,
    /// The compressed file, relative to the release.
    pub file: String,
    /// Size of the compressed file.
    pub size: u64,
    /// Size of the pack once unpacked.
    pub db_size: u64,
    /// SHA-256 of the unpacked pack.
    pub sha256: String,
    pub schema_version: String,
    pub built_at: String,
    pub entries: u64,
}

/// The packs available at `base` that this version of the app can read.
pub fn available(base: &str, ui: Lang) -> Result<Vec<RemotePack>, String> {
    let url = format!("{base}manifest.json");
    let text = ureq::get(&url)
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(|e| i18n::manifest_error(ui, e))?;
    let manifest: Manifest =
        serde_json::from_str(&text).map_err(|e| i18n::manifest_error(ui, e))?;
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err(i18n::manifest_error(
            ui,
            format!("manifest version {}", manifest.manifest_version),
        ));
    }
    Ok(manifest
        .packs
        .into_iter()
        .filter(|p| p.schema_version == SCHEMA_VERSION)
        .collect())
}

/// Counts the compressed bytes read, to report progress.
struct Counting<R> {
    inner: R,
    count: u64,
    on_progress: Box<dyn FnMut(u64) + Send>,
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count += n as u64;
        (self.on_progress)(self.count);
        Ok(n)
    }
}

/// Downloads `pack` from `base` and unpacks it into `dest`, checking its
/// checksum. `on_progress` receives the compressed bytes received so far.
pub fn download(
    base: &str,
    pack: &RemotePack,
    dest: &Path,
    ui: Lang,
    on_progress: impl FnMut(u64) + Send + 'static,
) -> Result<(), String> {
    let result = download_to(base, pack, dest, on_progress)
        .map_err(|e| i18n::download_error(ui, &pack.lang_name, e));
    let result = result.and_then(|sha256| {
        if sha256 == pack.sha256 {
            Ok(())
        } else {
            Err(i18n::checksum_error(ui, &pack.lang_name))
        }
    });
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}

/// Returns the SHA-256 of what was written.
fn download_to(
    base: &str,
    pack: &RemotePack,
    dest: &Path,
    on_progress: impl FnMut(u64) + Send + 'static,
) -> Result<String, Box<dyn std::error::Error>> {
    // The file name comes from the network: keep it a plain file name.
    if pack.file.contains(['/', '\\']) || pack.file.contains("..") {
        return Err(format!("unexpected file name {}", pack.file).into());
    }
    let url = format!("{base}{}", pack.file);
    let response = ureq::get(&url).call()?;
    let body = Counting {
        inner: response.into_body().into_reader(),
        count: 0,
        on_progress: Box::new(on_progress),
    };
    let mut unpacked = GzDecoder::new(body);
    let mut out = BufWriter::new(File::create(dest)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 1 << 16];
    loop {
        let n = unpacked.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }
    out.flush()?;
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// Serves `files` over HTTP on a local port; returns the base URL.
    fn serve(files: Vec<(&'static str, Vec<u8>)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut request = String::new();
                BufReader::new(&stream).read_line(&mut request).unwrap();
                let path = request
                    .split(' ')
                    .nth(1)
                    .unwrap_or("/")
                    .trim_start_matches('/');
                let response = match files.iter().find(|(name, _)| *name == path) {
                    Some((_, body)) => [
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .into_bytes(),
                        body.clone(),
                    ]
                    .concat(),
                    None => {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_vec()
                    }
                };
                let _ = stream.write_all(&response);
            }
        });
        base
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn manifest(sha256: &str, size: usize, schema: &str) -> Vec<u8> {
        format!(
            r#"{{"manifest_version": 1, "packs": [
                {{"lang": "fr", "lang_name": "Français", "file": "fr.db.gz", "size": {size},
                  "db_size": 5, "sha256": "{sha256}", "schema_version": "{schema}",
                  "built_at": "2026-10-22T00:00:00+00:00", "entries": 1}},
                {{"lang": "xx", "lang_name": "Future", "file": "xx.db.gz", "size": 1,
                  "db_size": 1, "sha256": "", "schema_version": "99",
                  "built_at": "", "entries": 0}}]}}"#
        )
        .into_bytes()
    }

    fn dest(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("qd-download-{}-{name}", std::process::id()))
    }

    #[test]
    fn downloads_unpacks_and_checks() {
        let pack = b"hello".to_vec();
        let sha = hex::encode(Sha256::digest(&pack));
        let compressed = gzip(&pack);
        let base = serve(vec![
            (
                "manifest.json",
                manifest(&sha, compressed.len(), SCHEMA_VERSION),
            ),
            ("fr.db.gz", compressed.clone()),
        ]);

        // Packs for a newer format are not offered.
        let available = available(&base, Lang::En).unwrap();
        assert_eq!(
            available
                .iter()
                .map(|p| p.lang.as_str())
                .collect::<Vec<_>>(),
            ["fr"]
        );

        let path = dest("ok");
        let seen = Arc::new(Mutex::new(0));
        let progress = seen.clone();
        download(&base, &available[0], &path, Lang::En, move |n| {
            *progress.lock().unwrap() = n
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), pack);
        assert_eq!(*seen.lock().unwrap(), compressed.len() as u64);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_a_damaged_download() {
        let compressed = gzip(b"hello");
        let base = serve(vec![
            (
                "manifest.json",
                manifest("0000", compressed.len(), SCHEMA_VERSION),
            ),
            ("fr.db.gz", compressed),
        ]);
        let pack = available(&base, Lang::En).unwrap().remove(0);
        let path = dest("damaged");
        let error = download(&base, &pack, &path, Lang::En, |_| {}).unwrap_err();
        assert!(error.contains("damaged"), "{error}");
        assert!(!path.exists(), "a damaged download is deleted");
    }

    #[test]
    fn reports_a_missing_manifest() {
        let base = serve(vec![]);
        let error = available(&base, Lang::Fr).unwrap_err();
        assert!(
            error.starts_with("Impossible d’obtenir la liste"),
            "{error}"
        );
    }

    #[test]
    fn refuses_file_names_with_paths() {
        let pack = RemotePack {
            lang: "fr".into(),
            lang_name: "Français".into(),
            file: "../evil".into(),
            size: 0,
            db_size: 0,
            sha256: String::new(),
            schema_version: SCHEMA_VERSION.into(),
            built_at: String::new(),
            entries: 0,
        };
        let error = download(
            "http://127.0.0.1:9/",
            &pack,
            &dest("evil"),
            Lang::En,
            |_| {},
        )
        .unwrap_err();
        assert!(error.contains("unexpected file name"), "{error}");
    }
}
