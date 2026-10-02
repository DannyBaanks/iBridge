#![cfg(target_os = "ios")]

use serde::Deserialize;
use tauri::{
    Manager, Runtime,
    plugin::{Builder, PluginHandle, TauriPlugin},
};

tauri::ios_plugin_binding!(init_plugin_ibridge_tunnel);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub connected: bool,
    pub status: String,
}

pub struct IBridgeTunnel<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> IBridgeTunnel<R> {
    pub fn start(&self) -> Result<TunnelStatus, tauri::plugin::mobile::PluginInvokeError> {
        self.0.run_mobile_plugin("start", ())
    }

    pub fn status(&self) -> Result<TunnelStatus, tauri::plugin::mobile::PluginInvokeError> {
        self.0.run_mobile_plugin("status", ())
    }

    pub fn stop(&self) -> Result<TunnelStatus, tauri::plugin::mobile::PluginInvokeError> {
        self.0.run_mobile_plugin("stop", ())
    }
}

pub trait IBridgeTunnelExt<R: Runtime> {
    fn ibridge_tunnel(&self) -> &IBridgeTunnel<R>;
}

impl<R: Runtime, T: Manager<R>> IBridgeTunnelExt<R> for T {
    fn ibridge_tunnel(&self) -> &IBridgeTunnel<R> {
        self.state::<IBridgeTunnel<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("ibridge-tunnel")
        .setup(|app, api| {
            let handle = api.register_ios_plugin(init_plugin_ibridge_tunnel)?;
            app.manage(IBridgeTunnel(handle));
            Ok(())
        })
        .build()
}
