//! Windows.
//!
//! The native "mode" of a file is its attributes (read-only, hidden, system,
//! archive), as in ZTreeWin. Archives and copies still deal in Unix
//! permissions, approximated from the read-only attribute.

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST,
    GetAdaptersAddresses, IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, FILE_ATTRIBUTE_OFFLINE,
    FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, FILE_ATTRIBUTE_TEMPORARY,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_WRITE_ATTRIBUTES,
    GetDiskFreeSpaceExW, GetFileInformationByHandle, GetLogicalDrives, SetFileAttributesW,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::{DiskSpace, Memory, Mount, Network, System, env_dir};

/// The attributes shown and edited by Xylem, in display order.
const RHSA: [(u32, char); 4] = [
    (FILE_ATTRIBUTE_READONLY, 'r'),
    (FILE_ATTRIBUTE_HIDDEN, 'h'),
    (FILE_ATTRIBUTE_SYSTEM, 's'),
    (FILE_ATTRIBUTE_ARCHIVE, 'a'),
];
const RHSA_MASK: u32 = FILE_ATTRIBUTE_READONLY
    | FILE_ATTRIBUTE_HIDDEN
    | FILE_ATTRIBUTE_SYSTEM
    | FILE_ATTRIBUTE_ARCHIVE;

/// The file attributes.
pub fn mode(meta: &Metadata) -> u32 {
    meta.file_attributes()
}

/// Unix-like permission bits, for archives and copies: 644 (755 for
/// directories), without the write bits when the file is read-only.
pub fn unix_mode(meta: &Metadata) -> u32 {
    let mode = if meta.is_dir() { 0o755 } else { 0o644 };
    if meta.permissions().readonly() {
        mode & !0o222
    } else {
        mode
    }
}

/// Attributes for an archive entry with Unix permissions.
pub fn from_unix_mode(unix: u32) -> u32 {
    if unix & 0o222 == 0 {
        FILE_ATTRIBUTE_READONLY
    } else {
        0
    }
}

/// Hidden by the H attribute (names starting with a dot are ordinary here).
pub fn is_hidden(_name: &OsStr, mode: u32) -> bool {
    mode & FILE_ATTRIBUTE_HIDDEN != 0
}

/// Programs are known by the extension.
pub fn is_executable(name: &OsStr, _mode: u32) -> bool {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some("exe" | "com" | "bat" | "cmd" | "ps1" | "msi")
    )
}

/// `r-sa`: one letter per attribute, `-` when off.
pub fn format_mode(mode: u32) -> String {
    RHSA.iter()
        .map(|&(bit, c)| if mode & bit != 0 { c } else { '-' })
        .collect()
}

/// Label and value for the Info screen.
pub fn describe_mode(mode: u32) -> (&'static str, String) {
    ("Attributes", format_mode(mode))
}

pub fn mode_prompt(what: &str) -> String {
    format!("ATTRIBUTES for {what} (RHSA, or +/-): ")
}

/// Starting text of the prompt: the attributes that are on (`RA`).
pub fn mode_text(mode: u32) -> String {
    RHSA.iter()
        .filter(|&&(bit, _)| mode & bit != 0)
        .map(|&(_, c)| c.to_ascii_uppercase())
        .collect()
}

/// `RA` sets exactly those attributes; `+R -A` turns some on or off and
/// keeps the rest. Other attributes (directory, compressed...) are kept.
pub fn parse_mode(text: &str, current: u32) -> Result<u32, String> {
    let error = || "Attributes are R, H, S and A, e.g. RA or +R -A".to_string();
    let relative = text.contains(['+', '-']);
    let mut mode = if relative {
        current
    } else {
        current & !RHSA_MASK
    };
    let mut on = true;
    for c in text.chars() {
        match c.to_ascii_lowercase() {
            '+' => on = true,
            '-' => on = false,
            ' ' => {}
            c => {
                let &(bit, _) = RHSA.iter().find(|&&(_, l)| l == c).ok_or_else(error)?;
                if on {
                    mode |= bit;
                } else {
                    mode &= !bit;
                }
            }
        }
    }
    Ok(mode)
}

pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let settable = RHSA_MASK
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED
        | FILE_ATTRIBUTE_OFFLINE
        | FILE_ATTRIBUTE_TEMPORARY;
    let attrs = match mode & settable {
        0 => FILE_ATTRIBUTE_NORMAL,
        a => a,
    };
    let wide = wide(path);
    // SAFETY: `wide` is NUL-terminated.
    if unsafe { SetFileAttributesW(wide.as_ptr(), attrs) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Applies Unix permissions: only the write bits matter (read-only).
pub fn set_file_mode(file: &File, unix: u32) -> io::Result<()> {
    let mut perms = file.metadata()?.permissions();
    perms.set_readonly(unix & 0o222 == 0);
    file.set_permissions(perms)
}

/// Changes the modification date: needs a handle opened with permission to
/// write attributes (and, for directories, backup semantics).
pub fn set_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .set_modified(time)
}

/// Deletes a file or link. Like `rm` on Linux, a read-only file is deleted
/// too; a link to a directory (or junction) is removed as a directory.
pub fn remove_file(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink_dir() {
        return fs::remove_dir(path);
    }
    if meta.permissions().readonly() {
        set_mode(path, mode(&meta) & !FILE_ATTRIBUTE_READONLY)?;
    }
    fs::remove_file(path)
}

/// Volume serial number and file index: identify a file regardless of the
/// name used to reach it. Links are not followed.
pub type FileId = (u32, u64);

pub fn file_id(path: &Path, _meta: &Metadata) -> Option<FileId> {
    let file = OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .ok()?;
    // SAFETY: plain C struct, filled in by the call below.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is open for the whole call; `info` is a local.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return None;
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((info.dwVolumeSerialNumber, index))
}

/// Volume of the file (to notice other volumes mounted inside a branch).
pub fn device(path: &Path, meta: &Metadata) -> u64 {
    file_id(path, meta).map_or(0, |(volume, _)| u64::from(volume))
}

/// Creates a symbolic link (needs Developer Mode or administrator rights).
pub fn symlink(target: &Path, link: &Path, is_dir: bool) -> io::Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Characters and names that Windows does not accept in file names.
pub fn check_name(name: &str) -> Result<(), String> {
    if let Some(c) = name
        .chars()
        .find(|&c| c < ' ' || r#"<>:"/\|?*"#.contains(c))
    {
        return Err(format!("Name cannot contain '{}'", c.escape_default()));
    }
    if name.ends_with(['.', ' ']) {
        return Err("Name cannot end with a dot or a space".into());
    }
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let reserved = ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|r| stem.eq_ignore_ascii_case(r))
        || (stem.len() == 4
            && ["COM", "LPT"]
                .iter()
                .any(|p| stem.as_bytes()[..3].eq_ignore_ascii_case(p.as_bytes()))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if reserved {
        return Err(format!("{stem} is a reserved name on Windows"));
    }
    Ok(())
}

/// Files under the user profile are already private.
pub fn private(options: &mut OpenOptions) -> &mut OpenOptions {
    options
}

/// Space a file takes on the disk. Windows would need the cluster size
/// and GetCompressedFileSizeW; the size is close enough for now.
pub fn disk_usage(meta: &Metadata) -> u64 {
    meta.len()
}

/// Hard links are not told apart on Windows (yet): none.
pub fn hard_link(_meta: &Metadata) -> Option<u64> {
    None
}

pub fn disk_space(path: &Path) -> Option<DiskSpace> {
    let wide = wide(path);
    let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: `wide` is NUL-terminated and the pointers are to local u64s.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    (ok != 0).then_some(DiskSpace {
        total,
        free,
        available,
    })
}

/// The drives (`C:\`, `D:\`...).
pub fn mounts() -> Vec<Mount> {
    // SAFETY: no arguments; returns a bit mask, A = bit 0.
    let mask = unsafe { GetLogicalDrives() };
    (0..26)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| Mount {
            source: String::new(),
            point: PathBuf::from(format!("{}:\\", char::from(b'A' + i as u8))),
            fstype: String::new(),
        })
        .collect()
}

/// Owners on Windows would take the security APIs: not shown.
pub fn owner(_path: &Path) -> Option<String> {
    None
}

pub fn home_dir() -> Option<PathBuf> {
    env_dir("USERPROFILE")
}

/// `%APPDATA%` (roaming: goes with the user to other machines).
pub(super) fn config_base() -> Option<PathBuf> {
    env_dir("APPDATA")
}

/// `%LOCALAPPDATA%`.
pub(super) fn state_base() -> Option<PathBuf> {
    env_dir("LOCALAPPDATA")
}

/// `%LOCALAPPDATA%\xylem\cache`.
pub(super) fn cache_base() -> Option<PathBuf> {
    Some(env_dir("LOCALAPPDATA")?.join(super::DIR_NAME).join("cache"))
}

/// Not looked for on Windows.
pub(super) fn fish_data_dir() -> Option<PathBuf> {
    None
}

/// The machine, from the registry and the memory and process APIs.
pub fn system() -> System {
    const VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let build = registry_string(VERSION, "CurrentBuild");
    let os = registry_string(VERSION, "ProductName").map(|name| {
        // Windows 11 still calls itself "Windows 10" in ProductName.
        let eleven = build.as_deref().and_then(|b| b.parse::<u32>().ok()) >= Some(22000);
        let name = if eleven {
            name.replacen("Windows 10", "Windows 11", 1)
        } else {
            name
        };
        match registry_string(VERSION, "DisplayVersion") {
            Some(v) => format!("{name} {v}"),
            None => name,
        }
    });
    let kernel = build.map(|b| match registry_dword(VERSION, "UBR") {
        Some(ubr) => format!("Windows NT build {b}.{ubr}"),
        None => format!("Windows NT build {b}"),
    });
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `status` is a valid MEMORYSTATUSEX with its size filled in.
    let memory = (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(Memory {
        total: status.ullTotalPhys,
        available: status.ullAvailPhys,
    });
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let size = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the pseudo handle of the current process and a buffer of `size` bytes.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) } != 0;
    System {
        os,
        kernel,
        host: std::env::var("COMPUTERNAME").ok(),
        cpu: registry_string(
            r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
            "ProcessorNameString",
        ),
        // SAFETY: no arguments.
        uptime: Some(Duration::from_millis(unsafe { GetTickCount64() })),
        load: None,
        memory,
        // The page file is not swap in the Unix sense; left out.
        swap: None,
        xt_memory: ok.then_some(counters.WorkingSetSize as u64),
    }
}

/// The adapters that are up, with their addresses, gateway and DNS, from
/// `GetAdaptersAddresses`.
pub fn network() -> Network {
    let mut net = Network::default();
    let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
    // u64 to align the structures the API writes in the buffer.
    let mut buf: Vec<u64> = Vec::new();
    let mut size: u32 = 16 * 1024;
    for _ in 0..3 {
        buf.resize((size as usize).div_ceil(8), 0);
        // SAFETY: a buffer of at least `size` bytes; on overflow, `size`
        // comes back with what is needed.
        let err = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                std::ptr::null(),
                buf.as_mut_ptr().cast(),
                &mut size,
            )
        };
        match err {
            0 => break,
            ERROR_BUFFER_OVERFLOW => continue,
            _ => return net,
        }
    }
    let mut next = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    // SAFETY: a linked list written by the API inside `buf`, which outlives
    // the loop; every pointer in it is null or valid.
    while let Some(a) = unsafe { next.as_ref() } {
        next = a.Next;
        if a.OperStatus != IfOperStatusUp || a.IfType == IF_TYPE_SOFTWARE_LOOPBACK {
            continue;
        }
        let name = unsafe { from_wide_ptr(a.FriendlyName) };
        let mut u = a.FirstUnicastAddress;
        while let Some(addr) = unsafe { u.as_ref() } {
            if let Some(ip) = unsafe { socket_ip(&addr.Address) } {
                net.add(&name, ip, addr.OnLinkPrefixLength);
            }
            u = addr.Next;
        }
        let mut g = a.FirstGatewayAddress;
        while let Some(addr) = unsafe { g.as_ref() } {
            // The IPv4 gateway is preferred, as `ipconfig` shows it first.
            if let Some(ip) = unsafe { socket_ip(&addr.Address) }
                && net.gateway.is_none_or(|old| old.is_ipv6() && ip.is_ipv4())
            {
                net.gateway = Some(ip);
            }
            g = addr.Next;
        }
        let mut d = a.FirstDnsServerAddress;
        while let Some(addr) = unsafe { d.as_ref() } {
            if let Some(ip) = unsafe { socket_ip(&addr.Address) }
                && !net.dns.contains(&ip)
            {
                net.dns.push(ip);
            }
            d = addr.Next;
        }
    }
    net
}

/// The IP of a SOCKET_ADDRESS (IPv4 or IPv6).
///
/// # Safety
/// `lpSockaddr` is null or points to a sockaddr of its family.
unsafe fn socket_ip(s: &SOCKET_ADDRESS) -> Option<IpAddr> {
    // SAFETY: as the caller guarantees.
    unsafe {
        let family = s.lpSockaddr.as_ref()?.sa_family;
        match family {
            AF_INET => {
                let sin = &*(s.lpSockaddr as *const SOCKADDR_IN);
                // S_addr is in network byte order, as are its bytes in memory.
                Some(Ipv4Addr::from(sin.sin_addr.S_un.S_addr.to_ne_bytes()).into())
            }
            AF_INET6 => {
                let sin6 = &*(s.lpSockaddr as *const SOCKADDR_IN6);
                Some(Ipv6Addr::from(sin6.sin6_addr.u.Byte).into())
            }
            _ => None,
        }
    }
}

/// A NUL-terminated UTF-16 string.
///
/// # Safety
/// `p` is null or points to a NUL-terminated string.
unsafe fn from_wide_ptr(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: as the caller guarantees.
    unsafe {
        let len = (0..).take_while(|&i| *p.add(i) != 0).count();
        String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
    }
}

/// A text value under HKEY_LOCAL_MACHINE.
fn registry_string(key: &str, value: &str) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut bytes = size_of_val(&buf) as u32;
    let (key, value) = (wide(Path::new(key)), wide(Path::new(value)));
    // SAFETY: NUL-terminated names and a buffer of `bytes` bytes.
    let err = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if err != 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let text = String::from_utf16_lossy(&buf[..len]).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// A DWORD value under HKEY_LOCAL_MACHINE.
fn registry_dword(key: &str, value: &str) -> Option<u32> {
    let mut data = 0u32;
    let mut bytes = size_of::<u32>() as u32;
    let (key, value) = (wide(Path::new(key)), wide(Path::new(value)));
    // SAFETY: NUL-terminated names and a 4-byte buffer.
    let err = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut data).cast(),
            &mut bytes,
        )
    };
    (err == 0).then_some(data)
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        let ra = FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_ARCHIVE;
        assert_eq!(format_mode(ra), "r--a");
        assert_eq!(mode_text(ra), "RA");
        assert_eq!(
            parse_mode("hs", ra),
            Ok(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
        );
        assert_eq!(
            parse_mode("+h -r", ra),
            Ok(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_ARCHIVE)
        );
        assert_eq!(parse_mode("", ra), Ok(0));
        assert!(parse_mode("x", ra).is_err());
        // The directory bit survives.
        assert_eq!(parse_mode("r", 0x10), Ok(0x10 | FILE_ATTRIBUTE_READONLY));
    }

    #[test]
    fn sets_and_reads_attributes_and_deletes_read_only() {
        let tmp = crate::testing::tempdir();
        let f = tmp.path().join("a.txt");
        fs::write(&f, "x").unwrap();
        set_mode(&f, FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN).unwrap();
        let m = mode(&fs::metadata(&f).unwrap());
        assert_eq!(format_mode(m), "rh--");
        assert!(is_hidden(OsStr::new("a.txt"), m));
        remove_file(&f).unwrap();
        assert!(!f.exists());
    }

    #[test]
    fn file_ids() {
        let tmp = crate::testing::tempdir();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        let id = |p: &Path| file_id(p, &fs::metadata(p).unwrap()).unwrap();
        assert_eq!(id(&a), id(&tmp.path().join(".").join("a")));
        assert_ne!(id(&a), id(&b));
    }

    #[test]
    fn names() {
        assert!(check_name("ok.txt").is_ok());
        assert!(check_name("a:b").is_err());
        assert!(check_name("nul.txt").is_err());
        assert!(check_name("COM1").is_err());
        assert!(check_name("COM10").is_ok());
        assert!(check_name("end.").is_err());
        assert!(is_executable(OsStr::new("Setup.EXE"), 0));
        assert!(!is_executable(OsStr::new("notes.txt"), 0));
    }

    #[test]
    fn system_on_windows() {
        let s = system();
        assert!(s.os.unwrap().starts_with("Windows"));
        assert!(s.kernel.unwrap().starts_with("Windows NT build"));
        assert!(s.memory.unwrap().total > 0);
        assert!(s.xt_memory.unwrap() > 0);
        // Only that it does not fail: CI machines may have no network.
        let _ = network();
    }
}
