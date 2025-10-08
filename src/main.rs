#[macro_use]
extern crate windows_service;
use std::ffi::OsString;
use windows_service::service_dispatcher;
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use std::time::Duration;
use windows_service::service::{
    ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,ServiceControl
};
use std::sync::mpsc::{self, channel};
use std::thread;
use crate::define::ErrorMess;
use windows::Win32::{Networking::WinSock::*};
mod define;
mod server;
use std::sync::Arc;


define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(arguments: Vec<OsString>) {
    if let Err(_e) = run_service(arguments) {
        panic!(" lỗi {_e:?}");
    }
}

fn run_service(_arguments: Vec<OsString>) ->Result<(), ErrorMess> { //windows_service::Result<()> 
    define::dbg_print(format!("runservice").to_string().as_str());
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => {
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(0);
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::UserEvent(code) =>{
                if code.to_raw() == 130 {
                    let  _ = shutdown_tx.send(0);
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register("myservice", event_handler)?;


    let next_status1 = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP, 
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(_e) = status_handle.set_service_status(next_status1){
        define::dbg_print(format!("{_e:?}").to_string().as_str());
    };

    let sock:Arc<SOCKET> = Arc::new(server::init_socket()?);
    let server_addr: SOCKADDR_IN = server::create_server_addr();
    server::bind_socket(*sock, server_addr)?;
    server::setup_tcp_listen(*sock)?;
    let sock_for_thread = Arc::clone(&sock);
    let handles = thread::spawn(move || -> Result<(), ErrorMess> {
        loop {
            match server::accept_client(*sock_for_thread){
                Ok(client_sock) => {
                    thread::spawn(move || {
                        if let Err(e) = server::receive_mess(client_sock) {
                            define::dbg_print(&format!("Client error: {:?}", e));
                        }
                    });
                },
                Err(_e) => {
                    define::dbg_print(format!("{_e:?}").to_string().as_str());
                    break;
                }
            };
        }
        Ok(())
    });

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    if let Err(_e) = status_handle.set_service_status(next_status){
        define::dbg_print(format!("{_e:?}").to_string().as_str());
        return Err(ErrorMess::StatusHandleError(format!("{_e:?}")))
    };

    loop {
        if let Ok(_) = shutdown_rx.recv() {
            if let Err(_e) = status_handle.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::StopPending,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 1,
                wait_hint: Duration::from_secs(10),
                process_id: None,
            }) {
                define::dbg_print(format!("Lỗi khi shutdown service {_e}").as_str());
                return Err(ErrorMess::StatusHandleError(format!("{_e:?}")));
            }
            server::_finish_socket(*sock)?;
            break;
        }
    }

    if let Err(_) = handles.join() {
        define::dbg_print("Thread accept bị panic hoặc không join được");
    }

    if let Err(_e) = status_handle.set_service_status( ServiceStatus { 
            service_type: ServiceType::OWN_PROCESS, 
            current_state: ServiceState::Stopped, 
            controls_accepted: ServiceControlAccept::empty(), 
            exit_code: ServiceExitCode::Win32(0), 
            checkpoint: 0, 
            wait_hint: Duration::default(), 
            process_id: None 
    }){
        define::dbg_print(format!("loi khi shutdown service 2 {_e}").to_string().as_str());
        return Err(ErrorMess::StatusHandleError(format!("{_e:?}")))
    }
    Ok(())
}
fn main() -> Result<(), windows_service::Error> {
    if let Err(e) = service_dispatcher::start("myservice", ffi_service_main) {
        eprintln!("Error starting service: {:?}", e);
    }
    Ok(())
}