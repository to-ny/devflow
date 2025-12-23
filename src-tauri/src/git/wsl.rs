//! WSL path handling. Routes git commands through wsl.exe for proper .gitignore support.

use std::path::Path;

#[cfg(windows)]
use std::process::{Command, Output};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
use log::info;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct WslPath {
    pub distro: String,
    pub linux_path: String,
}

#[cfg(windows)]
pub fn is_wsl_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.starts_with(r"\\wsl.localhost\") || path_str.starts_with(r"\\wsl$\")
}

#[cfg(not(windows))]
pub fn is_wsl_path(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
pub fn parse_wsl_path(path: &Path) -> Option<WslPath> {
    let path_str = path.to_string_lossy();
    let remainder = if let Some(r) = path_str.strip_prefix(r"\\wsl.localhost\") {
        r
    } else if let Some(r) = path_str.strip_prefix(r"\\wsl$\") {
        r
    } else {
        return None;
    };

    let parts: Vec<&str> = remainder.splitn(2, ['\\', '/']).collect();

    if parts.is_empty() {
        return None;
    }

    let distro = parts[0].to_string();
    if distro.is_empty() {
        return None;
    }

    let linux_path = if parts.len() > 1 {
        format!("/{}", parts[1].replace('\\', "/"))
    } else {
        "/".to_string()
    };

    Some(WslPath { distro, linux_path })
}

#[cfg(not(windows))]
pub fn parse_wsl_path(_path: &Path) -> Option<WslPath> {
    None
}

#[cfg(windows)]
pub fn run_git_via_wsl(wsl_path: &WslPath, args: &[&str]) -> std::io::Result<Output> {
    info!(
        "run_git_via_wsl: distro={}, path={}, args={:?}",
        wsl_path.distro, wsl_path.linux_path, args
    );

    let mut cmd = Command::new("wsl.exe");
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.args(["-d", &wsl_path.distro, "git", "-C", &wsl_path.linux_path]);
    cmd.args(args);

    cmd.output()
}

#[cfg(not(windows))]
pub fn run_git_via_wsl(
    _wsl_path: &WslPath,
    _args: &[&str],
) -> std::io::Result<std::process::Output> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "WSL is only available on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_is_wsl_path_modern() {
        assert!(is_wsl_path(Path::new(
            r"\\wsl.localhost\Ubuntu-22.04\home\tony"
        )));
        assert!(is_wsl_path(Path::new(r"\\wsl.localhost\Ubuntu\home")));
    }

    #[test]
    #[cfg(windows)]
    fn test_is_wsl_path_legacy() {
        assert!(is_wsl_path(Path::new(r"\\wsl$\Ubuntu-22.04\home\tony")));
        assert!(is_wsl_path(Path::new(r"\\wsl$\Ubuntu\home")));
    }

    #[test]
    fn test_is_wsl_path_negative() {
        assert!(!is_wsl_path(Path::new(r"/home/tony")));
        assert!(!is_wsl_path(Path::new(r"./relative/path")));
    }

    #[test]
    #[cfg(windows)]
    fn test_is_wsl_path_windows_paths() {
        assert!(!is_wsl_path(Path::new(r"C:\Users\tony")));
        assert!(!is_wsl_path(Path::new(r"\\server\share")));
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_modern() {
        let path = Path::new(r"\\wsl.localhost\Ubuntu-22.04\home\tony\dev\project");
        let parsed = parse_wsl_path(path).unwrap();

        assert_eq!(parsed.distro, "Ubuntu-22.04");
        assert_eq!(parsed.linux_path, "/home/tony/dev/project");
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_legacy() {
        let path = Path::new(r"\\wsl$\Ubuntu\home\tony");
        let parsed = parse_wsl_path(path).unwrap();

        assert_eq!(parsed.distro, "Ubuntu");
        assert_eq!(parsed.linux_path, "/home/tony");
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_root() {
        let path = Path::new(r"\\wsl.localhost\Ubuntu-22.04");
        let parsed = parse_wsl_path(path).unwrap();

        assert_eq!(parsed.distro, "Ubuntu-22.04");
        assert_eq!(parsed.linux_path, "/");
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_mixed_separators() {
        let path = Path::new(r"\\wsl.localhost\Ubuntu/home/tony/project");
        let parsed = parse_wsl_path(path).unwrap();

        assert_eq!(parsed.distro, "Ubuntu");
        assert_eq!(parsed.linux_path, "/home/tony/project");
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_special_distro_names() {
        let path = Path::new(r"\\wsl.localhost\Debian-11.5\var\log");
        let parsed = parse_wsl_path(path).unwrap();

        assert_eq!(parsed.distro, "Debian-11.5");
        assert_eq!(parsed.linux_path, "/var/log");
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_wsl_path_not_wsl() {
        let path = Path::new(r"C:\Users\tony");
        assert!(parse_wsl_path(path).is_none());
    }

    #[test]
    #[cfg(not(windows))]
    fn test_parse_wsl_path_non_windows() {
        assert!(parse_wsl_path(Path::new("/home/tony")).is_none());
    }
}
