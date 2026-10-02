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

pub const MOBILE_RELEASE_URL: &str =
    "https://github.com/DannyBaanks/iBridge/releases/latest/download/iBridge-Mobile.ipa";
const MOBILE_DISPLAY_NAME: &str = "iBridge";
pub const MOBILE_BUNDLE_PREFIX: &str = "me.dannybaanks.ibridge.mobile";
pub const MOBILE_PAIRING_PATH: &str = "iBridgeBootstrap/pairing.plist";
pub const MOBILE_BOOTSTRAP_PATH: &str = "iBridgeBootstrap/bootstrap.json";
pub const MOBILE_SECRET_PATH: &str = "iBridgeBootstrap/account-secret.json";
const MOBILE_RELEASE_FILENAME: &str = "iBridge-Mobile.ipa";

#[derive(Clone)]
pub struct MobileBootstrapAuth {
    pub email: String,
    pub password: String,
    pub anisette_server: String,
}

pub type MobileBootstrapAuthMutex = Mutex<Option<MobileBootstrapAuth>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileBootstrapPayload {
    schema: &'static str,
    device_udid: String,
    device_name: String,
    device_version: String,
    apple_id: String,
    anisette_server: String,
    pairing_path: &'static str,
    secret_path: &'static str,
    created_at: String,
}

impl MobileBootstrapPayload {
    pub fn new(
        device_udid: impl Into<String>,
        device_name: impl Into<String>,
        device_version: impl Into<String>,
        apple_id: impl Into<String>,
        anisette_server: impl Into<String>,
    ) -> Self {
        Self {
            schema: "ibridge.mobile-bootstrap/1",
            device_udid: device_udid.into(),
            device_name: device_name.into(),
            device_version: device_version.into(),
            apple_id: apple_id.into(),
            anisette_server: anisette_server.into(),
            pairing_path: MOBILE_PAIRING_PATH,
            secret_path: MOBILE_SECRET_PATH,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileAccountSecret {
    schema: &'static str,
    apple_id: String,
    password: String,
}

impl MobileAccountSecret {
    pub fn new(apple_id: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            schema: "ibridge.mobile-account-secret/1",
            apple_id: apple_id.into(),
            password: password.into(),
        }
    }
}

pub fn is_ibridge_mobile_bundle_id(bundle_id: &str) -> bool {
    bundle_id == MOBILE_BUNDLE_PREFIX
        || bundle_id
            .strip_prefix(MOBILE_BUNDLE_PREFIX)
            .is_some_and(|suffix| suffix.starts_with('.'))
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

        if is_ibridge_mobile_bundle_id(&bundle_id) || display_name == Some(MOBILE_DISPLAY_NAME) {
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
        .unwrap_or_else(|_| MOBILE_RELEASE_URL.to_string());
    let destination = handle
        .path()
        .temp_dir()
        .map_err(|e| AppError::Filesystem("Failed to get temp dir".into(), e.to_string()))?
        .join(MOBILE_RELEASE_FILENAME);
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

    let payload = MobileBootstrapPayload::new(
        device.info.udid,
        device.info.name,
        device.info.version,
        auth.email.clone(),
        auth.anisette_server,
    );
    let payload = serde_json::to_vec(&payload)
        .map_err(|e| AppError::Misc(format!("Failed to encode mobile bootstrap: {e}")))?;

    op.fail_if_err(
        "bootstrap",
        place_file(
            payload,
            &provider,
            bundle_id.clone(),
            MOBILE_BOOTSTRAP_PATH.to_string(),
        )
        .await,
    )?;

    let secret = MobileAccountSecret::new(auth.email, auth.password);
    let secret = serde_json::to_vec(&secret)
        .map_err(|e| AppError::Misc(format!("Failed to encode mobile account secret: {e}")))?;

    op.fail_if_err(
        "bootstrap",
        place_file(
            secret,
            &provider,
            bundle_id,
            MOBILE_SECRET_PATH.to_string(),
        )
        .await,
    )?;

    op.complete("bootstrap")?;
    Ok(())
}
