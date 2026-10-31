//! Folders mounted by rclone (`rclone mount`, Linux): which remote they
//! show and what service it is, from rclone's own configuration file, so
//! the DISK panel and Info can say "Google Drive" rather than just a path.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::duplicates::{self, Fail, Found, Notes, Progress};
use crate::platform::Mount;

/// A remote of rclone, as mounted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// What was mounted, as given to `rclone mount`: `gdrive:`,
    /// `s3:my-bucket`, `:s3:my-bucket`...
    pub spec: String,
    /// The service ("Google Drive", "Amazon S3"); `None` when rclone's
    /// configuration does not say (encrypted, or not found).
    pub service: Option<String>,
}

impl Remote {
    /// "gdrive: (Google Drive, through rclone)".
    pub fn describe(&self) -> String {
        match &self.service {
            Some(s) => format!("{} ({s}, through rclone)", self.spec),
            None => format!("{} (through rclone)", self.spec),
        }
    }
}

/// The rclone remote behind `mount`, if rclone mounted it.
pub fn of(mount: &Mount) -> Option<Remote> {
    if mount.fstype != "fuse.rclone" {
        return None;
    }
    let spec = mount.source.clone();
    // ":s3:bucket" is a remote made on the command line: its type is the
    // name. Otherwise the name goes up to the colon (or the comma of
    // "gdrive,shared_with_me:").
    let service = match spec.strip_prefix(':') {
        Some(rest) => {
            let kind = rest.split([':', ',']).next().unwrap_or_default();
            Some(service(kind, None))
        }
        None => {
            let name = spec.split([':', ',']).next().unwrap_or_default();
            service_of(name)
        }
    };
    Some(Remote { spec, service })
}

/// The name of `dir`, a folder under `point` where the remote `spec` is
/// mounted, as rclone calls it: `gdrive:` and `Backups/2023` make
/// `gdrive:Backups/2023`.
pub fn spec_of(spec: &str, point: &Path, dir: &Path) -> Option<String> {
    let inside = dir.strip_prefix(point).ok()?.to_str()?;
    if inside.is_empty() {
        return Some(spec.to_string());
    }
    let joint = if spec.ends_with(':') || spec.ends_with('/') {
        ""
    } else {
        "/"
    };
    Some(format!("{spec}{joint}{inside}"))
}

/// A file as `rclone lsjson` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Relative to the folder listed, and checked to stay inside it.
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub id: Option<String>,
    /// The strongest hash the service gave: its kind and value.
    pub hash: Option<(&'static str, String)>,
}

/// Strongest first. A kind is only compared with itself. CRC32 and the like
/// are too short to tell files apart, so they are not here.
const HASHES: [&str; 11] = [
    "sha256",
    "sha512",
    "blake3",
    "sha1",
    "whirlpool",
    "md5",
    "xxh128",
    "dropbox",
    "quickxor",
    "hidrive",
    "mailru",
];

#[derive(serde::Deserialize)]
struct Line {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "Size", default)]
    size: i64,
    #[serde(rename = "ModTime", default)]
    mod_time: String,
    #[serde(rename = "ID", default)]
    id: Option<String>,
    #[serde(rename = "Hashes", default)]
    hashes: HashMap<String, String>,
}

/// One line of the output of `rclone lsjson`, which has one file per line
/// between the brackets. `None` for what is not a file. A path that leaves the
/// folder listed is not taken: what a server says is hostile input as well.
pub fn parse_listed(line: &[u8]) -> Option<Listed> {
    let line = line.trim_ascii();
    let line = line.strip_suffix(b",").unwrap_or(line);
    if line == b"[" || line == b"]" {
        return None;
    }
    let line: Line = serde_json::from_slice(line).ok()?;
    let path = PathBuf::from(&line.path);
    if !path.components().all(|c| matches!(c, Component::Normal(_))) || line.path.contains('\\') {
        return None;
    }
    let hash = HASHES.iter().find_map(|kind| {
        let value = line.hashes.get(*kind)?.to_ascii_lowercase();
        (value.len() >= 16).then_some((*kind, value))
    });
    Some(Listed {
        path,
        // Documents of Google Docs and the like have no size: -1.
        size: u64::try_from(line.size).unwrap_or(0),
        modified: chrono::DateTime::parse_from_rfc3339(&line.mod_time)
            .ok()
            .map(SystemTime::from),
        id: line.id.filter(|id| !id.is_empty()),
        hash,
    })
}

/// The rclone running, shared with who may have to stop it.
#[derive(Clone, Default)]
pub struct Running(Arc<Mutex<Option<Child>>>);

impl Running {
    pub fn stop(&self) {
        if let Ok(mut child) = self.0.lock()
            && let Some(child) = child.as_mut()
        {
            let _ = child.kill();
        }
    }

    fn set(&self, child: Child) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(child);
        }
    }

    /// Waits for it to end, saying how. Not holding the lock while waiting:
    /// [`Running::stop`] is called from the screen's thread.
    fn wait(&self) -> bool {
        loop {
            {
                let Ok(mut slot) = self.0.lock() else {
                    return false;
                };
                let Some(child) = slot.as_mut() else {
                    return false;
                };
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *slot = None;
                        return status.success();
                    }
                    Ok(None) => {}
                    Err(_) => {
                        *slot = None;
                        return false;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// A line longer than this is not rclone's.
const MAX_LINE: u64 = 4 * 1024 * 1024;

/// Runs `rclone lsjson` on `spec` with `args`, giving `each` every file. An
/// error says what rclone said.
fn lsjson(
    program: &str,
    spec: &str,
    args: &[&str],
    running: &Running,
    cancel: &AtomicBool,
    mut each: impl FnMut(Listed),
) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(["lsjson", "-R", "--files-only", "--fast-list"])
        .args(args)
        .arg("--")
        .arg(spec)
        // Nothing of the folder being looked at can matter to it.
        .current_dir(crate::platform::home_dir().unwrap_or_else(|| PathBuf::from("/")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("{program} is not in the PATH")
            } else {
                format!("cannot run {program}: {e}")
            }
        })?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    running.set(child);
    if cancel.load(Ordering::Relaxed) {
        running.stop();
    }
    // Read on the side, or a chatty rclone would block on it: the last line
    // is what explains a failure.
    let errors = std::thread::spawn(move || {
        let mut last = String::new();
        if let Some(stderr) = stderr {
            for line in BufReader::new(stderr.take(64 * 1024 * 1024))
                .split(b'\n')
                .map_while(Result::ok)
            {
                if !line.is_empty() {
                    last = String::from_utf8_lossy(&line).into_owned();
                }
            }
        }
        last
    });
    let mut too_long = false;
    if let Some(stdout) = stdout {
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = reader.by_ref().take(MAX_LINE).read_until(b'\n', &mut line);
            match read {
                Ok(0) | Err(_) => break,
                Ok(n) if n as u64 == MAX_LINE => {
                    too_long = true;
                    running.stop();
                    break;
                }
                Ok(_) => {
                    if let Some(file) = parse_listed(&line) {
                        each(file);
                    }
                }
            }
        }
    }
    let success = running.wait();
    let last = errors.join().unwrap_or_default();
    if too_long {
        return Err(format!("{program} gave a line too long"));
    }
    if success {
        Ok(())
    } else {
        Err(format!("{program} failed: {last}"))
    }
}

/// Lists `spec` and groups its files by the hashes the service keeps. Nothing
/// is downloaded. [`Fail::NoHashes`] when that cannot be done.
#[allow(clippy::too_many_arguments)]
pub fn list_hashes(
    program: &str,
    remote: &Remote,
    spec: &str,
    dir: &Path,
    min_size: u64,
    ignore: &[String],
    progress: &Progress,
    cancel: &AtomicBool,
    running: &Running,
) -> Result<Found, Fail> {
    let drive = remote.service.as_deref() == Some("Google Drive");
    let mut args = vec!["--hash"];
    if drive {
        args.push("--drive-skip-shortcuts");
    }
    let mut listed = Vec::new();
    let mut notes = Notes::default();
    lsjson(program, spec, &args, running, cancel, |file| {
        progress.listed.fetch_add(1, Ordering::Relaxed);
        let ignored = file
            .path
            .parent()
            .into_iter()
            .flat_map(Path::components)
            .any(|c| ignore.iter().any(|i| c.as_os_str() == i.as_str()));
        if ignored || file.size < min_size.max(1) {
            // A document of Google Docs has no size and no hash.
            if !ignored && file.hash.is_none() {
                notes.no_hash += 1;
            }
            return;
        }
        if file.hash.is_none() {
            notes.no_hash += 1;
        }
        listed.push(file);
    })
    .map_err(Fail::NoHashes)?;
    if !listed.is_empty() && listed.iter().all(|f| f.hash.is_none()) {
        return Err(Fail::NoHashes(format!(
            "{} gives no hashes for its files",
            remote.spec
        )));
    }
    // Keep only what can have a twin: this can be millions of lines.
    let mut counts: HashMap<(&'static str, String, u64), u32> = HashMap::new();
    for file in &listed {
        if let Some((kind, hash)) = &file.hash {
            *counts.entry((*kind, hash.clone(), file.size)).or_default() += 1;
        }
    }
    listed.retain(|f| {
        f.hash.as_ref().is_some_and(|(kind, hash)| {
            counts
                .get(&(*kind, hash.clone(), f.size))
                .is_some_and(|n| *n > 1)
        })
    });
    // What others shared does not count on the user's own storage. Failing to
    // know only means they are not told apart.
    let mut shared = HashSet::new();
    if drive && !listed.is_empty() {
        let args = ["--drive-shared-with-me", "--no-modtime", "--no-mimetype"];
        let _ = lsjson(program, spec, &args, running, cancel, |file| {
            shared.extend(file.id);
        });
    }
    let files = duplicates::cloud_groups(dir, listed, &shared, &mut notes);
    Ok(Found {
        files,
        folders: Vec::new(),
        notes,
        cloud: Some(remote.clone()),
    })
}

/// The service of the remote `name`, read from rclone's configuration once
/// per name (the panel asks on every redraw).
fn service_of(name: &str) -> Option<String> {
    static KNOWN: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
    let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    known
        .get_or_insert_with(HashMap::new)
        .entry(name.to_string())
        .or_insert_with(|| {
            let text = config_file().and_then(|p| std::fs::read_to_string(p).ok())?;
            let (kind, provider) = remote_type(&text, name)?;
            Some(service(&kind, provider.as_deref()))
        })
        .clone()
}

/// rclone's configuration file: `$RCLONE_CONFIG`, else
/// `~/.config/rclone/rclone.conf` (or under `$XDG_CONFIG_HOME`), else the
/// old `~/.rclone.conf`.
fn config_file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RCLONE_CONFIG") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".config")));
    [
        config.map(|c| c.join("rclone/rclone.conf")),
        home.map(|h| h.join(".rclone.conf")),
    ]
    .into_iter()
    .flatten()
    .find(|p| p.is_file())
}

/// The `type` of the section `[name]` of an rclone configuration, and its
/// `provider` (S3) or `vendor` (WebDAV). `None` for an encrypted file or a
/// name that is not there.
fn remote_type(text: &str, name: &str) -> Option<(String, Option<String>)> {
    let mut inside = false;
    let (mut kind, mut provider) = (None, None);
    for line in text.lines().map(str::trim) {
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if inside {
                break;
            }
            inside = section == name;
        } else if inside && let Some((k, v)) = line.split_once('=') {
            let v = v.trim().to_string();
            match k.trim() {
                "type" => kind = Some(v),
                "provider" | "vendor" => provider = Some(v),
                _ => {}
            }
        }
    }
    kind.map(|k| (k, provider))
}

/// The name people know a storage type of rclone by.
fn service(kind: &str, provider: Option<&str>) -> String {
    let name = match (kind, provider.unwrap_or_default()) {
        ("drive", _) => "Google Drive",
        ("s3", "AWS") => "Amazon S3",
        ("s3", "Cloudflare") => "Cloudflare R2",
        ("s3", "Minio") => "MinIO",
        ("s3", "Wasabi") => "Wasabi",
        ("s3", "DigitalOcean") => "DigitalOcean Spaces",
        ("s3", "GCS") => "Google Cloud Storage",
        ("s3", "") => "S3",
        ("s3", other) => return format!("S3 ({other})"),
        ("webdav", "nextcloud") => "Nextcloud",
        ("webdav", "owncloud") => "ownCloud",
        ("webdav", "sharepoint" | "sharepoint-ntlm") => "SharePoint",
        ("webdav", _) => "WebDAV",
        ("onedrive", _) => "OneDrive",
        ("dropbox", _) => "Dropbox",
        ("b2", _) => "Backblaze B2",
        ("box", _) => "Box",
        ("pcloud", _) => "pCloud",
        ("mega", _) => "MEGA",
        ("protondrive", _) => "Proton Drive",
        ("iclouddrive", _) => "iCloud Drive",
        ("gcs" | "google cloud storage", _) => "Google Cloud Storage",
        ("gphotos" | "google photos", _) => "Google Photos",
        ("azureblob", _) => "Azure Blob Storage",
        ("azurefiles", _) => "Azure Files",
        ("swift", _) => "OpenStack Swift",
        ("crypt", _) => "encrypted (crypt)",
        ("sftp", _) => "SFTP",
        ("ftp", _) => "FTP",
        ("smb", _) => "SMB",
        ("local", _) => "local disk",
        (other, _) => return other.to_string(),
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "\
[gdrive]
type = drive
scope = drive
token = {\"access_token\":\"x\"}

[aws]
type = s3
provider = AWS
env_auth = true

[nuvem]
type = webdav
vendor = nextcloud
";

    #[test]
    fn types_from_the_configuration() {
        assert_eq!(remote_type(CONFIG, "gdrive"), Some(("drive".into(), None)));
        assert_eq!(
            remote_type(CONFIG, "aws"),
            Some(("s3".into(), Some("AWS".into())))
        );
        assert_eq!(
            remote_type(CONFIG, "nuvem"),
            Some(("webdav".into(), Some("nextcloud".into())))
        );
        assert_eq!(remote_type(CONFIG, "other"), None);
        let encrypted = "# Encrypted rclone configuration File\n\nRCLONE_ENCRYPT_V0:\nabc=";
        assert_eq!(remote_type(encrypted, "gdrive"), None);
    }

    #[test]
    fn service_names() {
        assert_eq!(service("drive", None), "Google Drive");
        assert_eq!(service("s3", Some("AWS")), "Amazon S3");
        assert_eq!(service("s3", Some("Ceph")), "S3 (Ceph)");
        assert_eq!(service("s3", None), "S3");
        assert_eq!(service("webdav", Some("nextcloud")), "Nextcloud");
        assert_eq!(service("newthing", None), "newthing");
    }

    const LISTING: &str = r#"[
{"Path":"Backups/a.zip","Name":"a.zip","Size":5000,"MimeType":"application/zip","ModTime":"2023-08-14T10:00:00.000Z","IsDir":false,"ID":"id1","Hashes":{"md5":"0123456789abcdef0123456789abcdef","sha1":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}},
{"Path":"Downloads/a (1).zip","Name":"a (1).zip","Size":5000,"ModTime":"2024-01-01T10:00:00Z","IsDir":false,"ID":"id2","Hashes":{"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},
{"Path":"Other/a.zip","Name":"a.zip","Size":5000,"ModTime":"2024-01-01T10:00:00Z","IsDir":false,"ID":"id1","Hashes":{"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},
{"Path":"Notes","Name":"Notes","Size":-1,"MimeType":"application/vnd.google-apps.document","ModTime":"2024-01-01T10:00:00Z","IsDir":false,"ID":"id3"},
{"Path":"../../etc/passwd","Name":"passwd","Size":5000,"IsDir":false,"ID":"id4","Hashes":{"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},
{"Path":"Shared/a.zip","Name":"a.zip","Size":5000,"IsDir":false,"ID":"id5","Hashes":{"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}
]
"#;

    fn parsed(text: &str) -> Vec<Listed> {
        text.lines()
            .filter_map(|l| parse_listed(l.as_bytes()))
            .collect()
    }

    #[test]
    fn a_listing_is_read() {
        let files = parsed(LISTING);
        assert_eq!(
            files.len(),
            5,
            "not the brackets, nor what leaves the folder"
        );
        assert_eq!(files[0].path, PathBuf::from("Backups/a.zip"));
        assert_eq!(files[0].size, 5000);
        assert_eq!(files[0].id.as_deref(), Some("id1"));
        assert!(files[0].modified.is_some());
        // The strongest hash, in small letters to compare with any other.
        let (kind, value) = files[0].hash.clone().unwrap();
        assert_eq!(
            (kind, value.as_str()),
            ("sha1", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        // A document has no hash and no size.
        assert_eq!(files[3].path, PathBuf::from("Notes"));
        assert_eq!(files[3].hash, None);
        assert_eq!(files[3].size, 0);
    }

    #[test]
    fn hostile_paths_are_refused() {
        for path in ["/etc/passwd", "a/../../b", "..", "./a", "a\\\\..\\\\b"] {
            let line = format!(r#"{{"Path":"{path}","Size":1}}"#);
            assert_eq!(parse_listed(line.as_bytes()), None, "{path}");
        }
    }

    #[test]
    fn groups_by_hash_with_the_drive_peculiarities() {
        let mut notes = Notes::default();
        let shared = HashSet::from(["id5".to_string()]);
        let groups =
            duplicates::cloud_groups(Path::new("/m"), parsed(LISTING), &shared, &mut notes);
        assert_eq!(groups.len(), 1);
        let paths: Vec<&Path> = groups[0].items.iter().map(|i| i.path.as_path()).collect();
        // The same ID twice is one file: Other/a.zip is not a copy of
        // Backups/a.zip.
        assert_eq!(paths.len(), 3);
        assert_eq!(notes.same_id, 1);
        assert!(paths.contains(&Path::new("/m/Backups/a.zip")));
        assert!(paths.contains(&Path::new("/m/Downloads/a (1).zip")));
        let shared_one = groups[0].items.iter().find(|i| i.shared).unwrap();
        assert_eq!(shared_one.path, Path::new("/m/Shared/a.zip"));
    }

    #[test]
    fn what_was_shared_is_not_tagged_by_all_but_one() {
        let mut notes = Notes::default();
        let shared = HashSet::from(["id5".to_string()]);
        let files = duplicates::cloud_groups(Path::new("/m"), parsed(LISTING), &shared, &mut notes);
        let found = Found {
            files,
            ..Found::default()
        };
        let mut view = duplicates::DuplicatesView::new(PathBuf::from("/m"), found);
        view.tag_all_but_one();
        let group = &view.groups()[0];
        assert!(group.items.iter().all(|i| !(i.shared && i.tagged)));
        assert_eq!(group.items.iter().filter(|i| i.tagged).count(), 1);
    }

    #[test]
    fn a_name_twice_in_one_folder_stays_once() {
        let line = |id: &str| {
            format!(
                r#"{{"Path":"f","Size":9,"ID":"{id}","Hashes":{{"md5":"0123456789abcdef0123456789abcdef"}}}}"#
            )
        };
        let files = vec![
            parse_listed(line("x").as_bytes()).unwrap(),
            parse_listed(line("y").as_bytes()).unwrap(),
        ];
        let mut notes = Notes::default();
        let groups = duplicates::cloud_groups(Path::new("/m"), files, &HashSet::new(), &mut notes);
        assert!(groups.is_empty());
        assert_eq!(notes.same_name, 1);
    }

    #[test]
    fn the_name_of_a_folder_for_rclone() {
        let point = Path::new("/home/u/gdrive");
        let at = |d: &str| spec_of("gdrive:", point, Path::new(d));
        assert_eq!(at("/home/u/gdrive").as_deref(), Some("gdrive:"));
        assert_eq!(at("/home/u/gdrive/a/b").as_deref(), Some("gdrive:a/b"));
        assert_eq!(at("/elsewhere"), None);
        let bucket = spec_of("s3:bucket", point, Path::new("/home/u/gdrive/a"));
        assert_eq!(bucket.as_deref(), Some("s3:bucket/a"));
    }

    /// A script in place of rclone: no network, and no rclone needed.
    #[cfg(unix)]
    fn fake_rclone(dir: &Path, body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake-rclone");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[cfg(unix)]
    fn run_fake(program: &str, service: Option<&str>) -> Result<Found, Fail> {
        let remote = Remote {
            spec: "gdrive:".into(),
            service: service.map(str::to_string),
        };
        list_hashes(
            program,
            &remote,
            "gdrive:",
            Path::new("/m"),
            1024,
            &[".git".to_string()],
            &Progress::default(),
            &AtomicBool::new(false),
            &Running::default(),
        )
    }

    #[cfg(unix)]
    #[test]
    fn rclone_is_run_and_listens_to_what_it_says() {
        let tmp = crate::testing::tempdir();
        let listing = tmp.path().join("listing");
        std::fs::write(&listing, LISTING).unwrap();
        // The second run is the list of what was shared: id5.
        let body = format!(
            r#"case "$*" in
  *shared-with-me*) printf '[\n{{"Path":"x","Size":1,"ID":"id5"}}\n]\n' ;;
  *) cat '{}' ;;
esac"#,
            listing.display()
        );
        let program = fake_rclone(tmp.path(), &body);
        let found = run_fake(&program, Some("Google Drive")).unwrap();
        assert_eq!(found.files.len(), 1);
        assert_eq!(found.notes.no_hash, 1, "the document");
        assert_eq!(found.files[0].items.iter().filter(|i| i.shared).count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn when_rclone_fails_or_is_missing_it_says_so() {
        let tmp = crate::testing::tempdir();
        let program = fake_rclone(tmp.path(), "echo 'bad remote' >&2; exit 3");
        let Err(Fail::NoHashes(why)) = run_fake(&program, None) else {
            panic!("should have failed");
        };
        assert!(why.ends_with("bad remote"), "{why}");
        let Err(Fail::NoHashes(why)) = run_fake("/nothing/here/rclone", None) else {
            panic!("should have failed");
        };
        assert!(why.contains("not in the PATH"), "{why}");
    }

    #[cfg(unix)]
    #[test]
    fn a_remote_without_hashes_is_told_apart() {
        let tmp = crate::testing::tempdir();
        let body = r#"printf '[\n{"Path":"a","Size":5000}\n,{"Path":"b","Size":5000}\n]\n'"#;
        let program = fake_rclone(tmp.path(), body);
        let Err(Fail::NoHashes(why)) = run_fake(&program, None) else {
            panic!("should have failed");
        };
        assert!(why.contains("no hashes"), "{why}");
    }

    #[test]
    fn only_rclone_mounts() {
        let mount = |source: &str, fstype: &str| Mount {
            source: source.into(),
            point: PathBuf::from("/home/user/gdrive"),
            fstype: fstype.into(),
        };
        assert_eq!(of(&mount("/dev/sda1", "ext4")), None);
        // Made on the command line: the type is in the name.
        let r = of(&mount(":s3:bucket/photos", "fuse.rclone")).unwrap();
        assert_eq!(r.service.as_deref(), Some("S3"));
        assert_eq!(r.describe(), ":s3:bucket/photos (S3, through rclone)");
        let r = of(&mount(":drive,shared_with_me:", "fuse.rclone")).unwrap();
        assert_eq!(r.service.as_deref(), Some("Google Drive"));
    }
}
