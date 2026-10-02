use std::{net::IpAddr, path::PathBuf, str::FromStr};

use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    pairing_file::PairingFile,
    provider::TcpProvider,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State, Window};

use crate::{
    account::{login_new, login_stored},
    error::AppError,
    mobile_bootstrap::{
        MOBILE_BOOTSTRAP_PATH, MOBILE_PAIRING_PATH, MOBILE_RELEASE_URL, MOBILE_SECRET_PATH,
        MobileAccountSecret, MobileBootstrapAuthMutex, MobileBootstrapPayload,
    },
    operation::Operation,
    sideload::{SideloaderGuard, SideloaderMutex, download},
};

pub const MOBILE_TUNNEL_FALLBACK_IP: &str = "10.7.0.1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileRuntimeStatus {
    pub pairing_ready: bool,
    pub tunnel_reachable: bool,
    pub tunnel_host: String,
}

fn document_path(handle: &AppHandle, relative: &str) -> Result<PathBuf, AppError> {
    handle
        .path()
        .document_dir()
        .map_err(|e| AppError::Filesystem("Failed to get app Documents directory".into(), e.to_string()))
        .map(|root| root.join(relative))
}

fn tunnel_host() -> String {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    if let Some(host) = discover_point_to_point_peer() {
        return host;
    }

    MOBILE_TUNNEL_FALLBACK_IP.to_string()
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn discover_point_to_point_peer() -> Option<String> {
    use std::{ffi::CStr, ptr};

    unsafe {
        let mut interfaces: *mut libc::ifaddrs = ptr::null_mut();
        if libc::getifaddrs(&mut interfaces) != 0 || interfaces.is_null() {
            return None;
        }

        struct Guard(*mut libc::ifaddrs);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { libc::freeifaddrs(self.0) };
            }
        }
        let _guard = Guard(interfaces);

        let mut current = interfaces;
        while !current.is_null() {
            let interface = &*current;
            current = interface.ifa_next;

            let flags = interface.ifa_flags as i32;
            if flags & libc::IFF_UP == 0 || flags & libc::IFF_POINTOPOINT == 0 {
                continue;
            }
            if interface.ifa_dstaddr.is_null() {
                continue;
            }

            let name = CStr::from_ptr(interface.ifa_name).to_string_lossy();
            if !name.starts_with("utun") {
                continue;
            }

            let address = interface.ifa_dstaddr;
            let family = (*address).sa_family as i32;
            let length = match family {
                libc::AF_INET => std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                libc::AF_INET6 => std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                _ => continue,
            };

            let mut host = [0_i8; libc::NI_MAXHOST as usize];
            if libc::getnameinfo(
                address,
                length,
                host.as_mut_ptr(),
                host.len() as libc::socklen_t,
                ptr::null_mut(),
                0,
                libc::NI_NUMERICHOST,
            ) != 0
            {
                continue;
            }

            let value = CStr::from_ptr(host.as_ptr()).to_string_lossy().into_owned();
            if value != "0.0.0.0" && value != "::" && !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

fn mobile_provider(handle: &AppHandle) -> Result<TcpProvider, AppError> {
    let pairing_path = document_path(handle, MOBILE_PAIRING_PATH)?;
    let pairing_bytes = std::fs::read(&pairing_path).map_err(|e| {
        AppError::Filesystem(
            "iBridge Mobile pairing bootstrap is missing".into(),
            format!("{}: {e}", pairing_path.display()),
        )
    })?;
    let pairing_file = PairingFile::from_bytes(&pairing_bytes)
        .map_err(|e| AppError::LockdownPairing("Failed to parse mobile pairing bootstrap".into(), e.to_string()))?;
    let tunnel_host = tunnel_host();
    let addr = IpAddr::from_str(&tunnel_host)
        .map_err(|e| AppError::Misc(format!("Invalid mobile tunnel address {tunnel_host}: {e}")))?;

    Ok(TcpProvider {
        addr,
        scope_id: None,
        pairing_file,
        label: "iBridge Mobile".into(),
    })
}

#[tauri::command]
pub async fn mobile_restore_bootstrap_account(
    handle: AppHandle,
    window: Window,
    sideloader_state: State<'_, SideloaderMutex>,
    mobile_auth_state: State<'_, MobileBootstrapAuthMutex>,
) -> Result<String, AppError> {
    let bootstrap_path = document_path(&handle, MOBILE_BOOTSTRAP_PATH)?;
    let bootstrap_bytes = std::fs::read(&bootstrap_path).map_err(|e| {
        AppError::Filesystem(
            "iBridge Mobile bootstrap metadata is missing".into(),
            format!("{}: {e}", bootstrap_path.display()),
        )
    })?;
    let bootstrap: MobileBootstrapPayload = serde_json::from_slice(&bootstrap_bytes)
        .map_err(|e| AppError::Misc(format!("Invalid iBridge Mobile bootstrap metadata: {e}")))?;

    let secret_path = document_path(&handle, MOBILE_SECRET_PATH)?;
    if secret_path.is_file() {
        let secret_bytes = std::fs::read(&secret_path).map_err(|e| {
            AppError::Filesystem(
                "Failed to read iBridge Mobile account bootstrap".into(),
                e.to_string(),
            )
        })?;
        let secret: MobileAccountSecret = serde_json::from_slice(&secret_bytes)
            .map_err(|e| AppError::Misc(format!("Invalid mobile account bootstrap: {e}")))?;
        if secret.apple_id != bootstrap.apple_id {
            return Err(AppError::Misc(
                "Mobile account bootstrap does not match bootstrap metadata".into(),
            ));
        }

        login_new(
            handle.clone(),
            window,
            sideloader_state,
            mobile_auth_state,
            secret.apple_id.clone(),
            secret.password,
            bootstrap.anisette_server,
            true,
        )
        .await?;

        std::fs::remove_file(&secret_path).map_err(|e| {
            AppError::Filesystem(
                "Account imported but one-time bootstrap secret could not be deleted".into(),
                e.to_string(),
            )
        })?;
        Ok(secret.apple_id)
    } else {
        login_stored(
            handle,
            window,
            bootstrap.apple_id.clone(),
            bootstrap.anisette_server,
            sideloader_state,
            mobile_auth_state,
        )
        .await?;
        Ok(bootstrap.apple_id)
    }
}

#[tauri::command]
pub async fn mobile_runtime_status(handle: AppHandle) -> Result<MobileRuntimeStatus, AppError> {
    let pairing_ready = document_path(&handle, MOBILE_PAIRING_PATH)?.is_file();
    let host = tunnel_host();
    if !pairing_ready {
        return Ok(MobileRuntimeStatus {
            pairing_ready: false,
            tunnel_reachable: false,
            tunnel_host: host,
        });
    }

    let provider = mobile_provider(&handle)?;
    let tunnel_reachable = LockdownClient::connect(&provider).await.is_ok();

    Ok(MobileRuntimeStatus {
        pairing_ready,
        tunnel_reachable,
        tunnel_host: host,
    })
}

async fn install_with_mobile_provider(
    handle: &AppHandle,
    sideloader_state: State<'_, SideloaderMutex>,
    app_path: String,
) -> Result<(), AppError> {
    let provider = mobile_provider(handle)?;
    let mut sideloader = SideloaderGuard::take(&sideloader_state)?;
    sideloader
        .get_mut()
        .install_app(
            &provider,
            app_path.into(),
            false,
            None::<fn(f32) -> std::future::Ready<()>>,
        )
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn mobile_sideload_operation(
    handle: AppHandle,
    window: Window,
    sideloader_state: State<'_, SideloaderMutex>,
    app_path: String,
) -> Result<(), AppError> {
    let op = Operation::new("mobile_sideload".to_string(), &window);
    op.start("install")?;
    op.fail_if_err(
        "install",
        install_with_mobile_provider(&handle, sideloader_state, app_path).await,
    )?;
    op.complete("install")?;
    Ok(())
}

#[tauri::command]
pub async fn mobile_refresh_self_operation(
    handle: AppHandle,
    window: Window,
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<(), AppError> {
    let op = Operation::new("mobile_refresh_self".to_string(), &window);
    op.start("download")?;

    let source = std::env::var("IBRIDGE_MOBILE_IPA_URL")
        .unwrap_or_else(|_| MOBILE_RELEASE_URL.to_string());
    let destination = handle
        .path()
        .temp_dir()
        .map_err(|e| AppError::Filesystem("Failed to get temp dir".into(), e.to_string()))?
        .join("iBridge-Mobile-refresh.ipa");
    op.fail_if_err("download", download(&source, &destination).await)?;

    op.move_on("download", "install")?;
    op.fail_if_err(
        "install",
        install_with_mobile_provider(
            &handle,
            sideloader_state,
            destination.to_string_lossy().to_string(),
        )
        .await,
    )?;
    op.complete("install")?;
    Ok(())
}
