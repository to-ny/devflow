use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use tauri::{AppHandle, Emitter};

const DEBOUNCE_MS: u64 = 500;
const POLL_INTERVAL_MS: u64 = 100;

pub struct WatcherService {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    shutdown: Arc<AtomicBool>,
}

impl WatcherService {
    pub fn new<P: AsRef<Path>>(app_handle: AppHandle, project_path: P) -> Result<Self, String> {
        let path = project_path.as_ref().to_path_buf();
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), tx)
            .map_err(|e| format!("Failed to create debouncer: {}", e))?;

        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        info!("WatcherService: watching {}", path.display());

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        std::thread::spawn(move || {
            Self::event_loop(rx, app_handle, path, shutdown_clone);
        });

        Ok(Self {
            _debouncer: debouncer,
            shutdown,
        })
    }

    fn event_loop(
        rx: Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
        app_handle: AppHandle,
        project_path: std::path::PathBuf,
        shutdown: Arc<AtomicBool>,
    ) {
        let timeout = Duration::from_millis(POLL_INTERVAL_MS);

        loop {
            if shutdown.load(Ordering::Relaxed) {
                info!("WatcherService: shutdown requested, stopping");
                break;
            }

            match rx.recv_timeout(timeout) {
                Ok(Ok(events)) => {
                    let has_relevant_changes = events.iter().any(|e| {
                        let path_str = e.path.to_string_lossy();
                        !path_str.contains(".git")
                            && matches!(
                                e.kind,
                                DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                            )
                    });

                    if has_relevant_changes {
                        info!("WatcherService: file changes detected");
                        if let Err(e) = app_handle.emit("files-changed", &project_path) {
                            warn!("Failed to emit files-changed event: {}", e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("WatcherService error: {}", e);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    info!("WatcherService: channel disconnected, stopping");
                    break;
                }
            }
        }
    }
}

impl Drop for WatcherService {
    fn drop(&mut self) {
        info!("WatcherService: dropping, signaling shutdown");
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
