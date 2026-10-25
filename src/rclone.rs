//! Folders mounted by rclone (`rclone mount`, Linux): which remote they
//! show and what service it is, from rclone's own configuration file, so
//! the DISK panel and Info can say "Google Drive" rather than just a path.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

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
