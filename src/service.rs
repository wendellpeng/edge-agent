use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

const SERVICE_NAME: &str = "EdgeAgent";

static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

/// Entry point called by main() on Windows.
/// If the process was launched by the Service Control Manager, this blocks until the service stops.
/// Otherwise returns ERROR_FAILED_SERVICE_CONTROLLER_CONNECT immediately.
pub fn run() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(_arguments: Vec<OsString>) {
    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    SHOULD_STOP.store(true, Ordering::Relaxed);
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        },
    )
    .expect("Failed to register service control handler");

    // Report Running to SCM.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .expect("Failed to set service status");

    // Run the agent inside a tokio runtime.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        match crate::init_agent().await {
            Ok(server) => {
                let handle = server.handle();
                let server_task = tokio::spawn(server);

                // Poll stop signal periodically.
                while !SHOULD_STOP.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }

                // Graceful shutdown.
                handle.stop(true).await;
                let _ = server_task.await;
            }
            Err(e) => {
                log::error!("Failed to start agent: {}", e);
            }
        }
    });

    // Report Stopped to SCM.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .expect("Failed to set service status");
}
