//! provider.iloader — device discovery through the iLoader-derived stack.
//!
//! Mirrors upstream `src-tauri/src/device.rs` usage of the `idevice` crate:
//! UsbmuxdConnection::default() -> get_devices() -> to_provider() ->
//! LockdownClient::connect() -> get_value(...). Read-only: no pairing, no
//! mutation, no signing. Communication is not authority.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct DeviceContract {
    pub schema: String,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
    pub host_alias: String,
    pub devices: Vec<DeviceInfoContract>,
    #[serde(rename = "usbmuxd")]
    pub transport: TransportState,
}

#[derive(Serialize, Clone)]
pub struct TransportState {
    pub reachable: bool,
    pub detail: String,
}

#[derive(Serialize, Clone)]
pub struct DeviceInfoContract {
    pub schema_version: u32,
    pub device_alias: String,
    pub stable_device_identifier: String,
    pub model: Option<String>,
    pub architecture: String,
    #[serde(rename = "iosVersion")]
    pub ios_version: Option<String>,
    #[serde(rename = "connectionType")]
    pub connection_type: String,
    pub capabilities: Vec<String>,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
    /// Diagnostics only (never promoted to a verdict): why enrichment failed.
    #[serde(rename = "diagnostic")]
    pub diagnostic: Option<String>,
}

pub async fn discover(host_alias: &str) -> DeviceContract {
    let observed_at = crate::core::iso_now();
    match run_discovery(host_alias, &observed_at).await {
        Ok((devices, detail)) => DeviceContract {
            schema: "ibridge.device.v1".into(),
            observed_at,
            host_alias: host_alias.to_string(),
            devices,
            transport: TransportState {
                reachable: true,
                detail,
            },
        },
        Err(e) => DeviceContract {
            schema: "ibridge.device.v1".into(),
            observed_at,
            host_alias: host_alias.to_string(),
            devices: vec![],
            transport: TransportState {
                reachable: false,
                detail: e,
            },
        },
    }
}

async fn run_discovery(
    host_alias: &str,
    observed_at: &str,
) -> Result<(Vec<DeviceInfoContract>, String), String> {
    use idevice::provider::UsbmuxdProvider;
    use idevice::usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection};

    // Respect the standard USBMUXD_SOCKET_ADDRESS env var for the INITIAL
    // connection (mirrors libimobiledevice tooling; upstream iLoader only
    // consults it for per-device providers). Without the var: same default.
    let mux_addr: UsbmuxdAddr = match std::env::var("USBMUXD_SOCKET_ADDRESS") {
        Ok(v) => UsbmuxdAddr::TcpSocket(
            v.parse()
                .map_err(|e| format!("bad USBMUXD_SOCKET_ADDRESS {v:?}: {e}"))?,
        ),
        Err(_) => UsbmuxdAddr::default(),
    };
    let mut usbmuxd: UsbmuxdConnection = mux_addr
        .connect(0)
        .await
        .map_err(|e| format!("usbmuxd unreachable: {e:?}"))?;

    let devs = usbmuxd
        .get_devices()
        .await
        .map_err(|e| format!("get_devices failed: {e:?}"))?;

    let detail = format!("{} device(s) enumerated via usbmuxd", devs.len());

    if devs.is_empty() {
        return Ok((vec![], detail));
    }

    let addr = UsbmuxdAddr::from_env_var()
        .map_err(|e| format!("invalid usbmuxd address from environment: {e:?}"))?;

    let surface = crate::core::device_capability_surface();
    let mut out = Vec::new();

    for d in &devs {
        let connection_type = match d.connection_type {
            Connection::Usb => "usb",
            Connection::Network(_) => "network",
            Connection::Unknown(_) => "unknown",
        }
        .to_string();

        let alias = crate::core::device_alias(host_alias, &d.udid);

        let (model, ios_version, diagnostic) =
            match enrich(&d.to_provider(addr.clone(), "ibridge")).await {
                Ok((m, v)) => (Some(m), Some(v), None),
                Err(e) => (None, None, Some(e)),
            };

        out.push(DeviceInfoContract {
            schema_version: 1,
            device_alias: alias.clone(),
            stable_device_identifier: alias,
            model,
            architecture: "arm64".into(),
            ios_version,
            connection_type,
            capabilities: surface.clone(),
            observed_at: observed_at.to_string(),
            diagnostic,
        });
    }
    Ok((out, detail))
}

/// Read-only lockdown enrichment: ProductType (model class) + ProductVersion.
/// Mirrors upstream; no pairing record is written, no session mutation.
async fn enrich(provider: &idevice::provider::UsbmuxdProvider) -> Result<(String, String), String> {
    use idevice::lockdown::LockdownClient;
    use idevice::IdeviceService;

    let mut lockdown = LockdownClient::connect(provider)
        .await
        .map_err(|e| format!("lockdown connect failed: {e:?}"))?;

    let model = lockdown
        .get_value(Some("ProductType"), None)
        .await
        .map_err(|e| format!("ProductType query failed: {e:?}"))?
        .as_string()
        .ok_or("ProductType not a string")?
        .to_string();

    let version = lockdown
        .get_value(Some("ProductVersion"), None)
        .await
        .map_err(|e| format!("ProductVersion query failed: {e:?}"))?
        .as_string()
        .ok_or("ProductVersion not a string")?
        .to_string();

    Ok((model, version))
}
