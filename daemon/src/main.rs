// ============================================================
// FILE:        main.rs
// MODULE:      Layer 1 — Enforcement Engine > Daemon Entry Point
// TASK:        T-010 (implementation — Session 2)
// PLATFORM:    windows | linux (macOS daemon is Swift-based)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, daemon core
// DEPENDENCIES: windows-service 0.7, tokio, tracing
// TEST COVERAGE: IT-01 (Windows service starts + blocks)
// KNOWN LIMITATIONS: [BLOCKED T-003] EV cert needed for production signing
// ============================================================

pub mod db;
mod hosts_manager;
mod ipc_server;
mod plan_protection;
mod process_monitor;
mod scheduler;
mod forced_mode;

#[cfg(windows)]
mod wfp_manager;

#[cfg(test)]
mod tests;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, warn};

use crate::db::Database;
use crate::hosts_manager::HostsManager;
use crate::ipc_server::IpcServer;
use crate::forced_mode::ForcedModeTracker;
use crate::process_monitor::ProcessMonitor;
use crate::scheduler::PlanScheduler;

/// Shared application state — passed to IPC handlers and subsystems
pub struct DaemonState {
    pub db: Arc<Database>,
    pub scheduler: Arc<PlanScheduler>,
    pub forced_mode: Arc<ForcedModeTracker>,
    pub process_monitor: Arc<ProcessMonitor>,
    pub hosts_manager: Arc<HostsManager>,
}

/// FocusMe Daemon — main entry point
///
/// On Windows: runs as a Windows Service via SCM
/// On Linux: runs as a systemd-managed daemon
/// On macOS: the Swift ESF daemon is the primary process (see macos/)
fn main() -> Result<()> {
    init_logging()?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "FocusMe Daemon starting"
    );

    #[cfg(windows)]
    {
        windows_service_main()?;
    }

    #[cfg(not(windows))]
    {
        run_daemon()?;
    }

    Ok(())
}

/// Initialize all daemon subsystems — called from both Windows Service
/// and Linux daemon code paths. Returns the DaemonState for IPC use.
async fn initialize_subsystems() -> Result<Arc<DaemonState>> {
    // 1. Open database (T-018)
    let db_path = Database::default_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let encryption_key = Database::derive_encryption_key();
    let db = Arc::new(Database::open(&db_path, &encryption_key)?);
    info!(path = %db_path.display(), "Policy store initialized");

    // 2. Create subsystems
    let hosts_manager = Arc::new(HostsManager::new()?);
    let process_monitor = Arc::new(ProcessMonitor::new());
    let forced_mode = Arc::new(ForcedModeTracker::new());
    let scheduler = Arc::new(PlanScheduler::new());

    // 3. Load plans from DB into scheduler
    let plan_rows = db.list_enabled_plans()?;
    let mut loaded_plans = Vec::new();
    for row in &plan_rows {
        let schedules_rows = db.get_schedules(&row.plan_id)?;
        let app_rules = db.get_app_rules(&row.plan_id)?;
        let url_rules = db.get_url_rules(&row.plan_id)?;

        loaded_plans.push(scheduler::LoadedPlan {
            plan_id: row.plan_id.clone(),
            name: row.name.clone(),
            enabled: row.enabled,
            forced_mode: row.forced_mode,
            schedules: schedules_rows.into_iter().map(|s| {
                scheduler::Schedule {
                    schedule_id: s.schedule_id,
                    plan_id: s.plan_id,
                    days: scheduler::parse_days_json(&s.days),
                    start_time: chrono::NaiveTime::parse_from_str(&s.start_time, "%H:%M")
                        .unwrap_or_else(|_| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
                    end_time: chrono::NaiveTime::parse_from_str(&s.end_time, "%H:%M")
                        .unwrap_or_else(|_| chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap()),
                    timezone: s.timezone.parse().unwrap_or(chrono_tz::UTC),
                }
            }).collect(),
            app_rules: app_rules.into_iter().map(|r| r.value).collect(),
            url_rules: url_rules.into_iter().map(|r| r.value).collect(),
        });
    }

    scheduler.load_plans(loaded_plans).await?;
    info!(plans = plan_rows.len(), "Plans loaded from database");

    // 4. Restore forced mode states from DB
    let fm_states = db.get_all_active_forced_modes()?;
    for fm in &fm_states {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&fm.expires_at_utc) {
            let now = chrono::Utc::now();
            if expires > now {
                let remaining = (expires - now).num_seconds().max(0) as u64;
                forced_mode.start_session(
                    fm.plan_id.clone(),
                    remaining,
                    fm.emergency_code_hash.clone(),
                ).await?;
                info!(plan_id = %fm.plan_id, remaining_s = remaining, "Forced mode restored from DB");
            } else {
                db.clear_forced_mode_state(&fm.plan_id)?;
            }
        }
    }

    let state = Arc::new(DaemonState {
        db,
        scheduler,
        forced_mode,
        process_monitor,
        hosts_manager,
    });

    Ok(state)
}

/// Start all background subsystem loops
async fn start_subsystems(state: &Arc<DaemonState>) -> Result<()> {
    // Start the scheduler loop (T-020)
    state.scheduler.start().await?;

    // Start process monitor polling loop (T-013)
    state.process_monitor.start().await?;

    // Start HOSTS file tamper detection (T-011)
    state.hosts_manager.start_tamper_detection().await?;

    // Restore HOSTS entries on startup
    state.hosts_manager.restore_entries().await?;

    info!("All subsystems started");
    Ok(())
}

/// Graceful shutdown of all subsystems
async fn shutdown_subsystems(state: &Arc<DaemonState>) {
    info!("Shutting down subsystems...");
    state.scheduler.stop().await;
    state.process_monitor.stop().await;
    if let Err(e) = state.hosts_manager.cleanup().await {
        warn!(error = %e, "Error cleaning up HOSTS file");
    }
    info!("All subsystems stopped");
}

/// Initialize tracing-based structured logging
fn init_logging() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().json().with_target(true))
        .with(filter)
        .init();

    Ok(())
}

// ============================================================
// WINDOWS SERVICE
// ============================================================

#[cfg(windows)]
fn windows_service_main() -> Result<()> {
    use windows_service::service_dispatcher;
    service_dispatcher::start("FocusMeDaemon", ffi_service_main)
        .map_err(|e| anyhow::anyhow!("Failed to start service dispatcher: {}", e))?;
    Ok(())
}

#[cfg(windows)]
extern "system" fn ffi_service_main(num_args: u32, args: *mut *mut u16) {
    if let Err(e) = run_service(num_args, args) {
        error!("Service failed: {}", e);
    }
}

#[cfg(windows)]
fn run_service(_num_args: u32, _args: *mut *mut u16) -> Result<()> {
    use windows_service::service::*;
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use std::sync::mpsc;

    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register("FocusMeDaemon", event_handler)
        .map_err(|e| anyhow::anyhow!("Failed to register service handler: {}", e))?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    }).map_err(|e| anyhow::anyhow!("Failed to set service status: {}", e))?;

    info!("FocusMe Windows Service running");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match initialize_subsystems().await {
            Ok(state) => {
                if let Err(e) = start_subsystems(&state).await {
                    error!(error = %e, "Failed to start subsystems");
                    return;
                }

                // Start IPC server (T-019) in background
                let ipc_state = state.clone();
                let ipc_handle = tokio::spawn(async move {
                    let ipc = IpcServer::new(false, ipc_state);
                    if let Err(e) = ipc.start().await {
                        error!(error = %e, "IPC server failed");
                    }
                });

                // Wait for SCM shutdown signal
                let _ = shutdown_rx.recv();
                info!("Shutdown signal received from SCM");

                shutdown_subsystems(&state).await;
                ipc_handle.abort();
            }
            Err(e) => {
                error!(error = %e, "Failed to initialize subsystems");
            }
        }
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    }).map_err(|e| anyhow::anyhow!("Failed to set stopped status: {}", e))?;

    Ok(())
}

// ============================================================
// LINUX / macOS DAEMON
// ============================================================

#[cfg(not(windows))]
fn run_daemon() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match initialize_subsystems().await {
            Ok(state) => {
                if let Err(e) = start_subsystems(&state).await {
                    error!(error = %e, "Failed to start subsystems");
                    return;
                }

                // Start IPC server in background
                let ipc_state = state.clone();
                let ipc_handle = tokio::spawn(async move {
                    let ipc = IpcServer::new(false, ipc_state);
                    if let Err(e) = ipc.start().await {
                        error!(error = %e, "IPC server failed");
                    }
                });

                info!("FocusMe Linux daemon running — press Ctrl+C to stop");
                tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
                info!("Shutdown signal received");

                shutdown_subsystems(&state).await;
                ipc_handle.abort();
            }
            Err(e) => {
                error!(error = %e, "Failed to initialize daemon");
            }
        }
    });

    Ok(())
}

// ============================================================
// UNIT TESTS
// ============================================================
#[cfg(test)]
mod tests {
    #[test]
    fn test_build_timestamp_env_var() {
        // Verify the build timestamp was embedded
        let ts = env!("BUILD_TIMESTAMP");
        assert!(!ts.is_empty());
    }
}
