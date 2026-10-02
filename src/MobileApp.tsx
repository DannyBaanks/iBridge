import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

import "./App.css";
import { AppleID } from "./AppleID";
import { GlassCard } from "./components/GlassCard";
import OperationView from "./components/OperationView";
import {
  Operation,
  OperationState,
  OperationUpdate,
} from "./components/operations";
import logo from "./iloader.svg";

const mobileSideloadOperation: Operation = {
  id: "mobile_sideload",
  titleKey: "operations.mobile_sideload_title",
  steps: [{ id: "install", titleKey: "operations.mobile_sideload_step_install" }],
};

const mobileRefreshOperation: Operation = {
  id: "mobile_refresh_self",
  titleKey: "operations.mobile_refresh_title",
  successTitleKey: "operations.mobile_refresh_success_title",
  steps: [
    { id: "download", titleKey: "operations.mobile_refresh_step_download" },
    { id: "install", titleKey: "operations.mobile_refresh_step_install" },
  ],
};

type RuntimeStatus = {
  pairingReady: boolean;
  tunnelReachable: boolean;
  tunnelHost: string;
};

type TunnelControlStatus = {
  connected: boolean;
  status: string;
};

const delay = (milliseconds: number) =>
  new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));

export default function MobileApp() {
  const [loggedInAs, setLoggedInAs] = useState<string | null>(null);
  const [noKeyringAvailable, setNoKeyringAvailable] = useState(false);
  const [status, setStatus] = useState<RuntimeStatus | null>(null);
  const [tunnelState, setTunnelState] = useState<string>("starting");
  const [restoring, setRestoring] = useState(true);
  const [operationState, setOperationState] = useState<OperationState | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const next = await invoke<RuntimeStatus>("mobile_runtime_status");
      setStatus(next);
      if (next.tunnelReachable) setTunnelState("connected");
    } catch (error) {
      console.error("Unable to read mobile runtime status", error);
    }
  }, []);

  const startTunnel = useCallback(async () => {
    try {
      const tunnel = await invoke<TunnelControlStatus>("mobile_tunnel_start");
      setTunnelState(tunnel.status);
      for (const wait of [250, 500, 1000]) {
        await delay(wait);
        const next = await invoke<RuntimeStatus>("mobile_runtime_status");
        setStatus(next);
        if (next.tunnelReachable) {
          setTunnelState("connected");
          return;
        }
      }
    } catch (error) {
      console.error("Unable to start iBridge Tunnel", error);
      setTunnelState("disconnected");
    }
  }, []);

  const startOperation = useCallback(
    async (operation: Operation, params: Record<string, unknown>) => {
      setOperationState({
        current: operation,
        started: [],
        failed: [],
        completed: [],
      });
      const unlisten = await listen<OperationUpdate>(
        `operation_${operation.id}`,
        (event) => {
          setOperationState((old) => {
            if (!old) return old;
            if (event.payload.updateType === "started") {
              return { ...old, started: [...old.started, event.payload.stepId] };
            }
            if (event.payload.updateType === "finished") {
              return { ...old, completed: [...old.completed, event.payload.stepId] };
            }
            return {
              ...old,
              failed: [
                ...old.failed,
                {
                  stepId: event.payload.stepId,
                  extraDetails: event.payload.extraDetails,
                },
              ],
            };
          });
        },
      );

      try {
        await invoke(`${operation.id}_operation`, params);
        await refreshStatus();
      } finally {
        unlisten();
      }
    },
    [refreshStatus],
  );

  useEffect(() => {
    invoke<boolean>("keyring_available")
      .then((available) => setNoKeyringAvailable(!available))
      .catch(() => setNoKeyringAvailable(true));

    refreshStatus();
    startTunnel();
  }, [refreshStatus, startTunnel]);

  useEffect(() => {
    let cancelled = false;
    const restore = async () => {
      try {
        const account = await invoke<string>("mobile_restore_bootstrap_account");
        if (!cancelled) setLoggedInAs(account);
      } catch (error) {
        console.error("Unable to restore mobile bootstrap account", error);
      } finally {
        if (!cancelled) setRestoring(false);
      }
    };
    restore();
    return () => {
      cancelled = true;
    };
  }, []);

  const installIpa = useCallback(async () => {
    if (!loggedInAs) {
      toast.error("Connect your Apple ID first.");
      return;
    }
    if (!status?.pairingReady) {
      toast.error("Connect this iPhone to iBridge Desktop once to finish setup.");
      return;
    }
    if (!status.tunnelReachable) {
      await startTunnel();
      const next = await invoke<RuntimeStatus>("mobile_runtime_status");
      setStatus(next);
      if (!next.tunnelReachable) {
        toast.error("iBridge Tunnel could not connect. Tap Start Tunnel and try again.");
        return;
      }
    }

    const path = await openFileDialog({
      multiple: false,
      filters: [{ name: "IPA Files", extensions: ["ipa"] }],
    });
    if (!path) return;

    await startOperation(mobileSideloadOperation, { appPath: path as string });
  }, [loggedInAs, status, startOperation, startTunnel]);

  return (
    <main className="workspace">
      <header className="workspace-header">
        <div className="header-left">
          <div className="title-block">
            <img src={logo} alt="iBridge" className="logo" />
            <div>
              <h1 className="title">iBridge Mobile</h1>
              <p className="subtitle">Choose an IPA. iBridge handles the rest.</p>
            </div>
          </div>
        </div>
      </header>

      <div className="workspace-body">
        <section className="workspace-content">
          <section className="workspace-section">
            <p className="section-label">Ready</p>
            <GlassCard className="panel">
              <div className="workspace-list">
                <div className="workspace-list-item">
                  Device setup: {status?.pairingReady ? "Ready" : "Needs desktop setup"}
                </div>
                <div className="workspace-list-item">
                  Local tunnel: {status?.tunnelReachable ? "Connected" : tunnelState}
                </div>
              </div>
              {!status?.tunnelReachable && (
                <div className="action-row">
                  <button onClick={() => startTunnel().catch(console.error)}>
                    Start Tunnel
                  </button>
                </div>
              )}
            </GlassCard>
          </section>

          <section className="workspace-section">
            <p className="section-label">Apple ID</p>
            <GlassCard className="panel">
              {restoring && <p>Restoring desktop setup…</p>}
              <AppleID
                loggedInAs={loggedInAs}
                setLoggedInAs={setLoggedInAs}
                noKeyringAvailable={noKeyringAvailable}
              />
            </GlassCard>
          </section>

          <section className="workspace-section">
            <p className="section-label">Apps</p>
            <GlassCard className="panel">
              <div className="action-row single-row">
                <button onClick={() => installIpa().catch(console.error)}>
                  Select IPA & Install
                </button>
                <button
                  onClick={() =>
                    startOperation(mobileRefreshOperation, {}).catch(console.error)
                  }
                  disabled={!loggedInAs || !status?.tunnelReachable}
                >
                  Refresh iBridge
                </button>
                <button onClick={() => refreshStatus().catch(console.error)}>
                  Check Connection
                </button>
              </div>
            </GlassCard>
          </section>

          {operationState && (
            <OperationView
              operationState={operationState}
              closeMenu={() => setOperationState(null)}
            />
          )}
        </section>
      </div>
    </main>
  );
}
