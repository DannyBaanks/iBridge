use std::sync::Mutex;

use chrono::Utc;
use idevice::{IdeviceService, installation_proxy::InstallationProxyClient};
use serde::Serialize;
use tauri::{AppHandle, Manager, State, Window};

use crate::{
    device::{DeviceInfo, DeviceInfoMutex, get_provider},
    error::AppError,
    operation::Operation,
    pairing::place_file,
    sideload::{SideloaderMutex, download, sideload},
};

const DEFAULT_MOBILE_IPA_URL: &str =
    "https://github.com/DannyBaanks/iBridge/releases/latest/download/iBridge-Mobile.ipa";
const MOBILE_DISPLAY_NAME: &str = "iBridge";
const MOBILE_BUNDLE_PREFIX: &str = "com.dannybaanks.ibridge.mobile";
const MOBILE_PAIRING_PATH: &str = "bootstrap/pairing.plist";
const MOBILE_BOOTSTRAP_PATH: &str = "bootstrap/iBridgeBootstrap.json";

#[derive(Clone)]
pub struct MobileBootstrapAuth {
    pub email: String,
    pub anisette_server: String,
}

pub type MobileBootstrapAuthMutex = Mutex<Option<MobileBootstrapAuth>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MobileBootstrapPayload {
    version: u8,
    apple_id: String,
    anisette_server: String,
    device_udid: String,
    device_name: String,
    device_version: String,
    created_at: String,
}

async fn installed_mobile_bundle_id(device: &DeviceInfo) -> Result<String, AppError> {
    let provider = get_provider(device).await?;
    let mut installation_proxy = InstallationProxyClient::connect(&provider)
        .await
        .map_err(|e| {
            AppError::DeviceComsWithMessage(
                "Failed to connect to installation proxy".into(),
                e.to_string(),
            )
        })?;

    let installed_apps = installation_proxy
        .get_apps(Some("User"), None)
        .await
        .map_err(|e| {
            AppError::DeviceComsWithMessage("Failed to get installed apps".into(), e.to_string())
        })?;

    for (bundle_id, app) in installed_apps {
        let dictionary = match app.as_dictionary() {
            Some(value) => value,
            None => continue,
        };
        let display_name = dictionary
            .get("CFBundleDisplayName")
            .or_else(|| dictionary.get("CFBundleName"))
            .and_then(|value| value.as_string());

        if display_name == Some(MOBILE_DISPLAY_NAME) || bundle_id.starts_with(MOBILE_BUNDLE_PREFIX) {
            return Ok(bundle_id);
        }
    }

    Err(AppError::Misc(
        "iBridge Mobile was installed but its bundle could not be found".into(),
    ))
}

#[tauri::command]
pub async fn install_ibridge_mobile_operation(
    handle: AppHandle,
    window: Window,
    device_state: State<'_, DeviceInfoMutex>,
    sideloader_state: State<'_, SideloaderMutex>,
    mobile_auth_state: State<'_, MobileBootstrapAuthMutex>,
) -> Result<(), AppError> {
    let op = Operation::new("install_ibridge_mobile".to_string(), &window);

    let device = {
        let guard = device_state.lock().unwrap();
        guard.clone().ok_or(AppError::NoDeviceSelected)?
    };
    let auth = {
        let guard = mobile_auth_state.lock().unwrap();
        guard.clone().ok_or(AppError::NotLoggedIn)?
    };

    op.start("download")?;
    let mobile_ipa_url = std::env::var("IBRIDGE_MOBILE_IPA_URL")
        .unwrap_or_else(|_| DEFAULT_MOBILE_IPA_URL.to_string());
    let destination = handle
        .path()
        .temp_dir()
        .map_err(|e| AppError::Filesystem("Failed to get temp dir".into(), e.to_string()))?
        .join("iBridge-Mobile.ipa");
    op.fail_if_err("download", download(&mobile_ipa_url, &destination).await)?;

    op.move_on("download", "install")?;
    op.fail_if_err(
        "install",
        sideload(
            device_state,
            sideloader_state,
            destination.to_string_lossy().to_string(),
        )
        .await,
    )?;

    op.move_on("install", "bootstrap")?;
    let bundle_id = op.fail_if_err("bootstrap", installed_mobile_bundle_id(&device.info).await)?;
    let provider = op.fail_if_err("bootstrap", get_provider(&device.info).await)?;

    op.fail_if_err(
        "bootstrap",
        place_file(
            device.pairing.clone(),
            &provider,
            bundle_id.clone(),
            MOBILE_PAIRING_PATH.to_string(),
        )
        .await,
    )?;

    let payload = MobileBootstrapPayload {
        version: 1,
        apple_id: auth.email,
        anisette_server: auth.anisette_server,
        device_udid: device.info.udid,
        device_name: device.info.name,
        device_version: device.info.version,
        created_at: Utc::now().to_rfc3339(),
    };
    let payload = serde_json::to_vec(&payload)
        .map_err(|e| AppError::Misc(format!("Failed to encode mobile bootstrap: {e}")))?;

    op.fail_if_err(
        "bootstrap",
        place_file(
            payload,
            &provider,
            bundle_id,
            MOBILE_BOOTSTRAP_PATH.to_string(),
        )
        .await,
    )?;

    op.complete("bootstrap")?;
    Ok(())
}
