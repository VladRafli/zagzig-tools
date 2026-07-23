use std::sync::{Arc, Mutex};
use std::time::Duration;

use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use self_update::update::ReleaseUpdate;

// How often to silently re-check in the background, once the initial
// startup check has settled. Matches the desktop app's interval.
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

const REPO_OWNER: &str = "VladRafli";
const REPO_NAME: &str = "zagzig-tools";
const BIN_NAME: &str = "zagzig-tui";

// Raw 32-byte Ed25519 verifying key generated with `zipsign gen-key` (see
// .github/workflows/release.yml, which signs zagzig-tui-*.zip with the
// matching private key — kept only as the ZIPSIGN_PRIVATE_KEY repo secret,
// never committed). This is a separate keypair from the desktop app's Tauri
// updater signing key; the two update mechanisms don't share trust.
const ZIPSIGN_PUBLIC_KEY: [u8; 32] = *include_bytes!("zipsign-public.key");

#[derive(Clone)]
pub struct AvailableUpdate {
    pub version: String,
}

#[derive(Default)]
struct Shared {
    checking: bool,
    available: Option<AvailableUpdate>,
    installing: bool,
    // Set once install_update() succeeds; the running process still has the
    // old code in memory (the binary on disk was replaced, but this process
    // already loaded it), so the app has to be restarted to actually run it.
    installed_version: Option<String>,
    error: Option<String>,
}

// Checks/installs updates from the same GitHub releases this binary itself
// is published from (see .github/workflows/release.yml, which builds and
// attaches a zagzig-tui-<target>.zip asset alongside the desktop app's
// installers). Mirrors MonitorEngine: state lives behind a plain
// std::sync::Mutex so the synchronous ratatui render closure can read it,
// while the actual check/install runs on a background tokio task.
#[derive(Clone)]
pub struct UpdateState {
    shared: Arc<Mutex<Shared>>,
}

impl UpdateState {
    pub fn new() -> Self {
        let state = Self {
            shared: Arc::new(Mutex::new(Shared {
                checking: true,
                ..Default::default()
            })),
        };
        state.spawn_check();
        state.spawn_periodic_check();
        state
    }

    // Background hourly re-check. Only fires while idle/errored — if
    // available or installing is already set, checking again would either
    // be redundant or race an in-progress install, so it's skipped and
    // picked back up on the next tick.
    fn spawn_periodic_check(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(CHECK_INTERVAL).await;
                let should_check = {
                    let shared = state.shared.lock().unwrap();
                    !shared.checking && !shared.installing && shared.available.is_none()
                };
                if should_check {
                    state.check_now();
                }
            }
        });
    }

    fn spawn_check(&self) {
        let shared = self.shared.clone();
        tokio::spawn(async move {
            let outcome = tokio::task::spawn_blocking(check_latest_release)
                .await
                .unwrap_or_else(|err| Err(err.to_string()));

            let mut shared = shared.lock().unwrap();
            shared.checking = false;
            match outcome {
                Ok(update) => {
                    shared.available = update;
                    shared.error = None;
                }
                Err(err) => shared.error = Some(err),
            }
        });
    }

    pub fn available(&self) -> Option<AvailableUpdate> {
        self.shared.lock().unwrap().available.clone()
    }

    pub fn checking(&self) -> bool {
        self.shared.lock().unwrap().checking
    }

    pub fn installing(&self) -> bool {
        self.shared.lock().unwrap().installing
    }

    pub fn installed_version(&self) -> Option<String> {
        self.shared.lock().unwrap().installed_version.clone()
    }

    pub fn error(&self) -> Option<String> {
        self.shared.lock().unwrap().error.clone()
    }

    // Triggers a fresh check — used for the manual "check for updates" key
    // as well as the periodic background re-check. No-op if one's already
    // running.
    pub fn check_now(&self) {
        let mut shared = self.shared.lock().unwrap();
        if shared.checking {
            return;
        }
        shared.checking = true;
        shared.error = None;
        drop(shared);
        self.spawn_check();
    }

    // Downloads and installs the update in place (see self_replace's
    // rename-based swap, which works even on Windows while the old exe is
    // still running). No-op if a check hasn't found a newer release yet or
    // an install is already underway.
    pub fn start_install(&self) {
        {
            let mut shared = self.shared.lock().unwrap();
            if shared.installing || shared.available.is_none() {
                return;
            }
            shared.installing = true;
            shared.error = None;
        }

        let shared = self.shared.clone();
        tokio::spawn(async move {
            let outcome = tokio::task::spawn_blocking(install_latest_release)
                .await
                .unwrap_or_else(|err| Err(err.to_string()));

            let mut shared = shared.lock().unwrap();
            shared.installing = false;
            match outcome {
                Ok(version) => {
                    shared.available = None;
                    shared.installed_version = Some(version);
                }
                Err(err) => shared.error = Some(err),
            }
        });
    }
}

fn updater() -> self_update::errors::Result<Box<dyn ReleaseUpdate>> {
    Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!())
        // We drive our own confirmation/progress display in the TUI rather
        // than letting self_update write to stdout, which would corrupt the
        // alternate screen.
        .no_confirm(true)
        .show_output(false)
        .show_download_progress(false)
        // Rejects the download unless it's signed by our zipsign key — this
        // is what actually makes install_latest_release() trustworthy, as
        // opposed to just "downloaded over HTTPS from GitHub".
        .verifying_keys([ZIPSIGN_PUBLIC_KEY])
        .build()
}

fn check_latest_release() -> Result<Option<AvailableUpdate>, String> {
    let updater = updater().map_err(|err| err.to_string())?;
    let release = updater.get_latest_release().map_err(|err| err.to_string())?;

    let is_newer = self_update::version::bump_is_greater(cargo_crate_version!(), &release.version)
        .unwrap_or(false);

    Ok(is_newer.then(|| AvailableUpdate {
        version: release.version,
    }))
}

fn install_latest_release() -> Result<String, String> {
    let updater = updater().map_err(|err| err.to_string())?;
    let status = updater.update().map_err(|err| err.to_string())?;
    Ok(status.version().to_string())
}
