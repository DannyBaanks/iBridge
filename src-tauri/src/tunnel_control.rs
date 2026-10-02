use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelControlStatus {
    pub connected: bool,
    pub status: String,
}

#[tauri::command]
pub fn mobile_tunnel_start(handle: AppHandle) -> Result<TunnelControlStatus, AppError> {
    #[cfg(target_os = "ios")]
    {
        use tauri_plugin_ibridge_tunnel::IBridgeTunnelExt;
        let status = handle
            .ibridge_tunnel()
            .start()
            .map_err(|e| AppError::Misc(format!("Failed to start iBridge tunnel: {e}")))?;
        return Ok(TunnelControlStatus {
            connected: status.connected,
            status: status.status,
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        let _ = handle;
        Err(AppError::Misc(
            "The embedded iBridge tunnel is only available on iOS".into(),
        ))
    }
}

#[tauri::command]
pub fn mobile_tunnel_status(handle: AppHandle) -> Result<TunnelControlStatus, AppError> {
    #[cfg(target_os = "ios")]
    {
        use tauri_plugin_ibridge_tunnel::IBridgeTunnelExt;
        let status = handle
            .ibridge_tunnel()
            .status()
            .map_err(|e| AppError::Misc(format!("Failed to query iBridge tunnel: {e}")))?;
        return Ok(TunnelControlStatus {
            connected: status.connected,
            status: status.status,
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        let _ = handle;
        Err(AppError::Misc(
            "The embedded iBridge tunnel is only available on iOS".into(),
        ))
    }
}

#[tauri::command]
pub fn mobile_tunnel_stop(handle: AppHandle) -> Result<TunnelControlStatus, AppError> {
    #[cfg(target_os = "ios")]
    {
        use tauri_plugin_ibridge_tunnel::IBridgeTunnelExt;
        let status = handle
            .ibridge_tunnel()
            .stop()
            .map_err(|e| AppError::Misc(format!("Failed to stop iBridge tunnel: {e}")))?;
        return Ok(TunnelControlStatus {
            connected: status.connected,
            status: status.status,
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        let _ = handle;
        Err(AppError::Misc(
            "The embedded iBridge tunnel is only available on iOS".into(),
        ))
    }
}
