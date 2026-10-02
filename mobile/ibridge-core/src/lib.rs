use std::{
    ffi::{CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use futures::FutureExt;
use idevice::{pairing_file::PairingFile, provider::TcpProvider};
use isideload::{
    anisette::remote_v3::RemoteV3AnisetteProvider,
    auth::apple_account::{AppleAccount, TwoFactorCallbackResponse},
    dev::developer_session::DeveloperSession,
    sideload::{SideloaderBuilder, builder::MaxCertsBehavior, install::install_app},
    util::storage::SideloadingStorage,
};
use rootcause::prelude::*;
use serde::{Deserialize, Serialize};

const KEYCHAIN_SERVICE: &str = "com.dannybaanks.ibridge.mobile";
const KEYCHAIN_ACCOUNT: &str = "applePassword";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInstallRequest {
    apple_id: String,
    verification_code: Option<String>,
    anisette_server: String,
    input_ipa: String,
    storage_dir: String,
    pairing_base64: String,
    device_address: String,
    scope_id: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreResponse {
    ok: bool,
    code: String,
    message: String,
}

#[derive(Debug)]
struct CoreFailure {
    code: &'static str,
    message: String,
}

impl CoreFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
struct DirectoryStorage {
    root: PathBuf,
}

impl DirectoryStorage {
    fn new(root: impl AsRef<Path>) -> Result<Self, CoreFailure> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).map_err(|error| {
            CoreFailure::new(
                "storage_error",
                format!("Unable to create signing storage: {error}"),
            )
        })?;
        Ok(Self { root })
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        let encoded = key
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.root.join(encoded)
    }
}

impl SideloadingStorage for DirectoryStorage {
    fn store(&self, key: &str, value: &str) -> Result<(), Report> {
        std::fs::create_dir_all(&self.root).context("create iBridge signing storage")?;
        std::fs::write(self.path_for_key(key), value).context("write iBridge signing state")?;
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>, Report> {
        match std::fs::read_to_string(self.path_for_key(key)) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("read iBridge signing state"),
        }
    }

    fn delete(&self, key: &str) -> Result<(), Report> {
        match std::fs::remove_file(self.path_for_key(key)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("delete iBridge signing state"),
        }
    }
}

async fn sign_and_install(request: SignInstallRequest) -> Result<(), CoreFailure> {
    let _ = isideload::init();

    let credential = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|error| CoreFailure::new("keychain_error", error.to_string()))?
        .get_password()
        .map_err(|error| CoreFailure::new("credential_missing", error.to_string()))?;

    let storage_root = PathBuf::from(&request.storage_dir);
    let tfa_requested = Arc::new(AtomicBool::new(false));
    let tfa_requested_for_callback = Arc::clone(&tfa_requested);
    let verification_code = request.verification_code.clone();

    let tfa_callback = move |_params| {
        let code = verification_code.clone();
        let tfa_requested = Arc::clone(&tfa_requested_for_callback);
        async move {
            tfa_requested.store(true, Ordering::SeqCst);
            Ok(match code {
                Some(code) if !code.trim().is_empty() => {
                    TwoFactorCallbackResponse::SubmitCode(code.trim().to_string())
                }
                _ => TwoFactorCallbackResponse::Abort,
            })
        }
        .boxed()
    };

    let anisette_url = if request.anisette_server.starts_with("http") {
        request.anisette_server.clone()
    } else {
        format!("https://{}", request.anisette_server)
    };

    let anisette_storage = DirectoryStorage::new(storage_root.join("anisette"))?;
    let anisette = RemoteV3AnisetteProvider::default()
        .map_err(|error| CoreFailure::new("anisette_error", error.to_string()))?
        .set_serial_number("0".to_string())
        .set_storage(Box::new(anisette_storage))
        .set_url(&anisette_url);

    let login_result = AppleAccount::builder(&request.apple_id.to_lowercase())
        .anisette_provider(anisette)
        .login(&credential, Box::new(tfa_callback))
        .await;

    let mut account = match login_result {
        Ok(account) => account,
        Err(error)
            if tfa_requested.load(Ordering::SeqCst)
                && request.verification_code.as_deref().unwrap_or("").is_empty() =>
        {
            return Err(CoreFailure::new(
                "two_factor_required",
                "Apple requested a verification code",
            ));
        }
        Err(error) => {
            return Err(CoreFailure::new("apple_login_failed", error.to_string()));
        }
    };

    let developer_session = DeveloperSession::from_account(&mut account)
        .await
        .map_err(|error| CoreFailure::new("developer_session_failed", error.to_string()))?;

    let signing_storage = DirectoryStorage::new(storage_root.join("signing"))?;
    let mut sideloader = SideloaderBuilder::new(developer_session, request.apple_id.to_lowercase())
        .machine_name("iBridge Mobile".into())
        .storage(Box::new(signing_storage))
        .max_certs_behavior(MaxCertsBehavior::Error)
        .build();

    let (signed_app_path, _) = sideloader
        .sign_app(
            PathBuf::from(&request.input_ipa),
            None,
            false,
            None::<fn(f32) -> std::future::Ready<()>>,
        )
        .await
        .map_err(|error| CoreFailure::new("sign_failed", error.to_string()))?;

    let pairing_bytes = BASE64_STANDARD
        .decode(request.pairing_base64.as_bytes())
        .map_err(|error| CoreFailure::new("pairing_decode_failed", error.to_string()))?;
    let pairing_file = PairingFile::from_bytes(&pairing_bytes)
        .map_err(|error| CoreFailure::new("pairing_invalid", error.to_string()))?;
    let device_address: IpAddr = request
        .device_address
        .parse()
        .map_err(|error| CoreFailure::new("device_address_invalid", format!("{error}")))?;

    let provider = TcpProvider {
        addr: device_address,
        scope_id: request.scope_id,
        pairing_file,
        label: "iBridge Mobile self-host".to_string(),
    };

    install_app(&provider, &signed_app_path, |_| {})
        .await
        .map_err(|error| CoreFailure::new("install_failed", error.to_string()))?;

    Ok(())
}

fn response_json(response: CoreResponse) -> *mut c_char {
    let json = serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"code\":\"serialization_error\",\"message\":\"Unable to encode response\"}"
            .to_string()
    });
    CString::new(json).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibridge_sign_and_install(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return response_json(CoreResponse {
            ok: false,
            code: "invalid_request".into(),
            message: "Request pointer was null".into(),
        });
    }

    let request_json = unsafe { CStr::from_ptr(request_json) };
    let request: SignInstallRequest = match serde_json::from_slice(request_json.to_bytes()) {
        Ok(request) => request,
        Err(error) => {
            return response_json(CoreResponse {
                ok: false,
                code: "invalid_request".into(),
                message: error.to_string(),
            });
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            return response_json(CoreResponse {
                ok: false,
                code: "runtime_failed".into(),
                message: error.to_string(),
            });
        }
    };

    match runtime.block_on(sign_and_install(request)) {
        Ok(()) => response_json(CoreResponse {
            ok: true,
            code: "ok".into(),
            message: "Signed and installed".into(),
        }),
        Err(error) => response_json(CoreResponse {
            ok: false,
            code: error.code.into(),
            message: error.message,
        }),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibridge_string_free(value: *mut c_char) {
    if !value.is_null() {
        let _ = unsafe { CString::from_raw(value) };
    }
}
