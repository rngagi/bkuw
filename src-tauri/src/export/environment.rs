//! Local TeX inspection and explicitly requested installer handoff.
//! No lexical data enters the installer or the dependency probe.
use crate::error::{AppError, AppResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStatus {
    pub state: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub missing_files: Vec<String>,
    pub diagnostic_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Default)]
pub struct InstallerState {
    busy: AtomicBool,
    cancelled: AtomicBool,
}

pub struct InstallGuard<'a>(&'a InstallerState);
impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}
impl InstallerState {
    pub fn begin(&self) -> AppResult<InstallGuard<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| failure("latex_install_busy"))?;
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(InstallGuard(self))
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    fn check_cancelled(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(failure("latex_install_cancelled"))
        } else {
            Ok(())
        }
    }
}

pub fn required_files() -> Vec<String> {
    // The template is the single source of truth for direct TeX dependencies.
    include_str!("../../templates/latex/main.tex")
        .lines()
        .filter_map(|line| {
            let extension = if line.starts_with("\\usepackage") {
                "sty"
            } else if line.starts_with("\\documentclass") {
                "cls"
            } else {
                return None;
            };
            let name = line.split('{').nth(1)?.split('}').next()?;
            Some(format!("{name}.{extension}"))
        })
        .collect()
}

fn failure(code: &'static str) -> AppError {
    AppError::new(code, "The LaTeX environment operation did not complete.")
}

// File-backed output prevents a blocked pipe from deadlocking a probe.
fn run_probe(command: &mut Command, dir: &Path, seconds: u64) -> AppResult<(bool, String)> {
    let log = tempfile::tempfile().map_err(|_| failure("latex_environment"))?;
    let mut child = command
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|_| failure("latex_environment"))?)
        .stderr(log.try_clone().map_err(|_| failure("latex_environment"))?)
        .spawn()
        .map_err(|_| failure("latex_environment"))?;
    let waited = child.wait_timeout(Duration::from_secs(seconds));
    let status = match waited {
        Ok(Some(status)) => status,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(failure("latex_environment_timeout"));
        }
    };
    use std::io::{Seek, SeekFrom};
    let mut log = log;
    log.seek(SeekFrom::Start(0))
        .map_err(|_| failure("latex_environment"))?;
    let mut bytes = Vec::new();
    log.take(256 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("latex_environment"))?;
    Ok((
        status.success(),
        String::from_utf8_lossy(&bytes).into_owned(),
    ))
}

pub fn inspect(
    engine: Option<PathBuf>,
    sources: Option<Vec<(String, Vec<u8>)>>,
    diagnostics: &Path,
) -> AppResult<EnvironmentStatus> {
    let mut result = EnvironmentStatus {
        state: "missing".into(),
        path: None,
        version: None,
        missing_files: vec![],
        diagnostic_path: None,
    };
    let Some(engine) = engine else {
        return Ok(result);
    };
    result.path = Some(engine.to_string_lossy().into_owned());
    result.state = "unusable".into();
    let dir = tempfile::tempdir().map_err(|_| failure("latex_environment"))?;
    let version = run_probe(Command::new(&engine).arg("--version"), dir.path(), 10);
    match version {
        Ok((true, text)) if text.contains("XeTeX") => {
            result.version = text.lines().next().map(str::to_owned)
        }
        Err(error) if error.code == "latex_environment_timeout" => {
            result.state = "timedOut".into();
            return Ok(result);
        }
        _ => return Ok(result),
    }
    let kpse = engine
        .parent()
        .unwrap_or(Path::new("."))
        .join(if cfg!(windows) {
            "kpsewhich.exe"
        } else {
            "kpsewhich"
        });
    for file in required_files() {
        match run_probe(Command::new(&kpse).arg(&file), dir.path(), 5) {
            Ok((true, output)) if !output.trim().is_empty() => {}
            _ => result.missing_files.push(file),
        }
    }
    if !result.missing_files.is_empty() {
        result.state = "missingPackages".into();
        return Ok(result);
    }
    let Some(sources) = sources else {
        result.state = "fontsMissing".into();
        return Ok(result);
    };
    for (name, bytes) in sources {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| failure("latex_environment"))?;
        }
        fs::write(path, bytes).map_err(|_| failure("latex_environment"))?;
    }
    let compiled = run_probe(
        Command::new(&engine).args([
            "-no-shell-escape",
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-file-line-error",
            "probe.tex",
        ]),
        dir.path(),
        30,
    );
    match compiled {
        Ok((true, _)) if dir.path().join("probe.pdf").is_file() => result.state = "ready".into(),
        outcome => {
            result.state = if matches!(&outcome, Err(e) if e.code == "latex_environment_timeout") {
                "timedOut"
            } else {
                "probeFailed"
            }
            .into();
            fs::create_dir_all(diagnostics).map_err(|_| failure("latex_environment"))?;
            let log = diagnostics.join(format!("latex-probe-{}.log", uuid::Uuid::new_v4()));
            let bytes = fs::read(dir.path().join("probe.log")).unwrap_or_else(|_| match outcome {
                Ok((_, text)) => text.into_bytes(),
                _ => b"XeLaTeX dependency probe failed or timed out.".to_vec(),
            });
            fs::write(&log, bytes).map_err(|_| failure("latex_environment"))?;
            result.diagnostic_path = Some(log.to_string_lossy().into_owned());
        }
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct Installer {
    url: &'static str,
    sha256: &'static str,
    filename: &'static str,
}

// MacTeX digest: Homebrew/homebrew-cask Casks/m/mactex.rb, 2026.0324.
// Windows digest computed from the TUG historic archive; SHA-512 checked against its sidecar.
const MAC: Installer = Installer {
    url: "https://ctan.math.illinois.edu/systems/mac/mactex/mactex-20260324.pkg",
    sha256: "e30af0640f51979a95b9f54084456d1e16749b79e12e65bdebd630fb2795ff1e",
    filename: "mactex-20260324.pkg",
};
const WINDOWS: Installer = Installer {
    url: "https://ftp.math.utah.edu/pub/tex/historic/systems/texlive/2025/tlnet-final/install-tl-windows.exe",
    sha256: "649662985f654bd202dd3c8c1ed453c0bc9fcbcd5d107ff9b10cb73474d4b46c",
    filename: "install-tl-2025-windows.exe",
};
fn platform_installer() -> AppResult<Installer> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok(MAC)
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Ok(WINDOWS)
    } else {
        Err(failure("latex_install_unsupported"))
    }
}
fn verify(actual: &[u8], expected: &str) -> AppResult<()> {
    if hex::encode(actual) == expected {
        Ok(())
    } else {
        Err(failure("latex_install_integrity"))
    }
}

pub fn install(
    root: &Path,
    state: &InstallerState,
    report: &dyn Fn(InstallProgress),
) -> AppResult<()> {
    let _guard = state.begin()?;
    let installer = platform_installer()?;
    if super::find_xelatex().is_some() {
        return Err(failure("latex_install_existing"));
    }
    fs::create_dir_all(root).map_err(|_| failure("latex_install_filesystem"))?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| failure("latex_install_download"))?;
    let mut response = tauri::async_runtime::block_on(client.get(installer.url).send())
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| failure("latex_install_download"))?;
    if !response.status().is_success() {
        return Err(failure("latex_install_download"));
    }
    let total = response.content_length();
    let path = download_installer(
        root,
        installer,
        state,
        total,
        || {
            let result = tauri::async_runtime::block_on(response.chunk());
            state.check_cancelled()?;
            result
                .map(|chunk| chunk.map(|bytes| bytes.to_vec()))
                .map_err(|_| failure("latex_install_download"))
        },
        report,
    )?;
    state.check_cancelled()?;
    launch(&path)?;
    report(InstallProgress {
        phase: "waiting".into(),
        downloaded_bytes: total.unwrap_or(0),
        total_bytes: total,
    });
    Ok(())
}

fn download_installer(
    root: &Path,
    installer: Installer,
    state: &InstallerState,
    total: Option<u64>,
    mut next_chunk: impl FnMut() -> AppResult<Option<Vec<u8>>>,
    report: &dyn Fn(InstallProgress),
) -> AppResult<PathBuf> {
    let mut output =
        tempfile::NamedTempFile::new_in(root).map_err(|_| failure("latex_install_filesystem"))?;
    let mut hash = Sha256::new();
    let mut downloaded = 0;
    let mut last_report = Instant::now();
    report(InstallProgress {
        phase: "downloading".into(),
        downloaded_bytes: 0,
        total_bytes: total,
    });
    loop {
        state.check_cancelled()?;
        let Some(buffer) = next_chunk()? else {
            break;
        };
        state.check_cancelled()?;
        downloaded += buffer.len() as u64;
        if downloaded > 8 * 1024 * 1024 * 1024 {
            return Err(failure("latex_install_download"));
        }
        output
            .write_all(&buffer)
            .map_err(|_| failure("latex_install_filesystem"))?;
        hash.update(&buffer);
        if last_report.elapsed() >= Duration::from_millis(200) {
            report(InstallProgress {
                phase: "downloading".into(),
                downloaded_bytes: downloaded,
                total_bytes: total,
            });
            last_report = Instant::now();
        }
    }
    report(InstallProgress {
        phase: "verifying".into(),
        downloaded_bytes: downloaded,
        total_bytes: total,
    });
    verify(&hash.finalize(), installer.sha256)?;
    state.check_cancelled()?;
    output
        .flush()
        .map_err(|_| failure("latex_install_filesystem"))?;
    let directory = root.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&directory).map_err(|_| failure("latex_install_filesystem"))?;
    let path = directory.join(installer.filename);
    output
        .persist_noclobber(&path)
        .map_err(|_| failure("latex_install_filesystem"))?;
    Ok(path)
}

fn launch(path: &Path) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("/usr/bin/open")
            .arg(path)
            .status()
            .map_err(|_| failure("latex_install_launch"))?
            .success()
            .then_some(())
            .ok_or_else(|| failure("latex_install_launch"))
    }
    #[cfg(target_os = "windows")]
    {
        Command::new(path)
            .args([
                "-repository",
                "https://ftp.math.utah.edu/pub/tex/historic/systems/texlive/2025/tlnet-final",
            ])
            .spawn()
            .map(|_| ())
            .map_err(|_| failure("latex_install_launch"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        Err(failure("latex_install_unsupported"))
    }
}

fn script(installer: Installer, windows: bool) -> String {
    if windows {
        format!(
            r#"# bkuw: download, verify, and open the official full TeX Live installer.
$ErrorActionPreference = 'Stop'
$dir = Join-Path ([IO.Path]::GetTempPath()) ('bkuw-tex-' + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $dir | Out-Null
$file = Join-Path $dir '{filename}'
Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile $file -MaximumRedirection 0
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant() -ne '{hash}') {{ Remove-Item -LiteralPath $file; throw 'SHA-256 verification failed / SHA-256 驗證失敗' }}
Start-Process -FilePath $file -ArgumentList '-repository','https://ftp.math.utah.edu/pub/tex/historic/systems/texlive/2025/tlnet-final' -Wait
Write-Host 'Return to bkuw and check again / 返回 bkuw 重新檢查'
"#,
            filename = installer.filename,
            url = installer.url,
            hash = installer.sha256
        )
    } else {
        format!(
            r#"#!/bin/sh
# bkuw: download, verify, and open the official full MacTeX installer.
set -eu
dir=$(mktemp -d "${{TMPDIR:-/tmp}}/bkuw-tex.XXXXXX")
cd "$dir"
/usr/bin/curl --fail --proto '=https' --connect-timeout 20 --output '{filename}' '{url}'
printf '%s  %s\n' '{hash}' '{filename}' | /usr/bin/shasum -a 256 -c -
/usr/bin/open "$dir/{filename}"
printf '%s\n' 'Return to bkuw and check again / 返回 bkuw 重新檢查'
"#,
            filename = installer.filename,
            url = installer.url,
            hash = installer.sha256
        )
    }
}

pub fn save_guide(destination: &Path) -> AppResult<String> {
    if !destination.is_dir() {
        return Err(failure("latex_install_filesystem"));
    }
    let dir = destination.join(format!("bkuw-latex-setup-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&dir).map_err(|_| failure("latex_install_filesystem"))?;
    for (name, contents) in [
        (
            "install-windows.ps1",
            format!("\u{feff}{}", script(WINDOWS, true)),
        ),
        ("install-macos.sh", script(MAC, false)),
        (
            "README.md",
            include_str!("../../templates/latex/INSTALL.md").to_owned(),
        ),
    ] {
        fs::write(dir.join(name), contents).map_err(|_| failure("latex_install_filesystem"))?;
    }
    Ok(dir.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installer_is_exclusive_and_retryable() {
        let state = InstallerState::default();
        let guard = state.begin().unwrap();
        assert!(state.begin().is_err());
        state.cancel();
        assert!(state.check_cancelled().is_err());
        drop(guard);
        let _next = state.begin().unwrap();
        assert!(state.check_cancelled().is_ok());
    }
    #[test]
    fn corrupt_download_never_verifies() {
        assert!(verify(&Sha256::digest(b"corrupt"), MAC.sha256).is_err());
        assert!(verify(&Sha256::digest(b"ok"), &hex::encode(Sha256::digest(b"ok"))).is_ok());
    }
    #[test]
    fn missing_engine_and_template_dependencies() {
        assert_eq!(
            inspect(None, None, Path::new(".")).unwrap().state,
            "missing"
        );
        assert!(required_files().contains(&"fontspec.sty".into()));
        assert!(required_files().contains(&"article.cls".into()));
    }
    #[test]
    fn saves_both_scripts_without_overwriting_existing_files() {
        let root = tempfile::tempdir().unwrap();
        let first = save_guide(root.path()).unwrap();
        let second = save_guide(root.path()).unwrap();
        assert_ne!(first, second);
        let ps = fs::read_to_string(Path::new(&first).join("install-windows.ps1")).unwrap();
        assert!(ps.contains(WINDOWS.sha256));
        assert!(ps.starts_with('\u{feff}'));
        assert!(ps.contains("-UseBasicParsing"));
        assert!(ps.contains("-MaximumRedirection 0"));
        assert!(
            fs::read_to_string(Path::new(&first).join("install-macos.sh"))
                .unwrap()
                .contains(MAC.sha256)
        );
    }
    #[cfg(unix)]
    fn fake_tool(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn inspection_distinguishes_broken_engine_missing_packages_and_compile_failure() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("含空白 TeX");
        fs::create_dir(&dir).unwrap();
        let engine = dir.join("xelatex");
        fake_tool(&engine, "exit 1");
        assert_eq!(
            inspect(Some(engine.clone()), None, &dir).unwrap().state,
            "unusable"
        );
        fake_tool(&engine, "echo XeTeX");
        let missing = inspect(Some(engine.clone()), None, &dir).unwrap();
        assert_eq!(missing.state, "missingPackages");
        assert!(missing.missing_files.contains(&"fontspec.sty".into()));
        fake_tool(&dir.join("kpsewhich"), "echo /tex/package.sty");
        assert_eq!(
            inspect(Some(engine.clone()), None, &dir).unwrap().state,
            "fontsMissing"
        );
        fake_tool(
            &engine,
            "if [ \"$1\" = --version ]; then echo XeTeX; exit 0; fi\necho missing-transitive-dependency > probe.log\nexit 1",
        );
        let failed = inspect(Some(engine.clone()), Some(vec![]), &dir).unwrap();
        assert_eq!(failed.state, "probeFailed");
        assert!(
            fs::read_to_string(failed.diagnostic_path.unwrap())
                .unwrap()
                .contains("missing-transitive-dependency")
        );
        fake_tool(
            &engine,
            "if [ \"$1\" = --version ]; then echo XeTeX; exit 0; fi\nprintf '%%PDF' > probe.pdf",
        );
        assert_eq!(
            inspect(Some(engine), Some(vec![]), &dir).unwrap().state,
            "ready"
        );
    }
    #[cfg(unix)]
    #[test]
    fn inspection_times_out_and_reaps_an_unresponsive_engine() {
        let dir = tempfile::tempdir().unwrap();
        let engine = dir.path().join("xelatex");
        fake_tool(&engine, "exec sleep 30");
        assert_eq!(
            inspect(Some(engine), None, dir.path()).unwrap().state,
            "timedOut"
        );
    }
    #[test]
    fn interrupted_corrupt_and_cancelled_downloads_leave_no_installer() {
        let root = tempfile::tempdir().unwrap();
        let state = InstallerState::default();
        let _guard = state.begin().unwrap();
        let mut chunks = vec![
            Ok(Some(b"partial".to_vec())),
            Err(failure("latex_install_download")),
        ]
        .into_iter();
        assert!(
            download_installer(
                root.path(),
                MAC,
                &state,
                None,
                || chunks.next().unwrap(),
                &|_| {}
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let mut chunks = vec![Some(b"corrupt".to_vec()), None].into_iter();
        assert!(
            download_installer(
                root.path(),
                MAC,
                &state,
                None,
                || Ok(chunks.next().unwrap()),
                &|_| {}
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let error = download_installer(
            root.path(),
            MAC,
            &state,
            None,
            || {
                state.cancel();
                Ok(Some(b"partial".to_vec()))
            },
            &|_| {},
        )
        .unwrap_err();
        assert_eq!(error.code, "latex_install_cancelled");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
    #[test]
    fn verified_download_preserves_unicode_paths_and_reports_verification() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("安裝 files");
        fs::create_dir(&target).unwrap();
        let installer = Installer {
            url: MAC.url,
            filename: "test.pkg",
            sha256: "2689367b205c16ce32ed4200942b8b8b1e262dfc70d9bc9fbc77c49699a4f1df",
        };
        let state = InstallerState::default();
        let mut chunks = vec![Some(b"o".to_vec()), Some(b"k".to_vec()), None].into_iter();
        let phases = std::cell::RefCell::new(Vec::new());
        let path = download_installer(
            &target,
            installer,
            &state,
            Some(2),
            || Ok(chunks.next().unwrap()),
            &|progress| phases.borrow_mut().push(progress.phase),
        )
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), b"ok");
        assert_eq!(*phases.borrow(), vec!["downloading", "verifying"]);
    }
}
