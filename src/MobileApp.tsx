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
  titleKey: "operations.sideload_title",
  steps: [{ id: "install", titleKey: "operations.sideload_step_install" }],
};

const mobileRefreshOperation: Operation = {
  id: "mobile_refresh_self",
  titleKey: "operations.sideload_title",
  steps: [
    { id: "download", titleKey: "operations.install_ibridge_mobile_step_download" },
    { id: "install", titleKey: "operations.sideload_step_install" },
  ],
};

type RuntimeStatus = {
  pairingReady: boolean;
  tunnelReachable: boolean;
  tunnelHost: string;
};

export default function MobileApp() {
  const [loggedInAs, setLoggedInAs] = useState<string | null>(null);
  const [noKeyringAvailable, setNoKeyringAvailable] = useState(false);
  const [status, setStatus] = useState<RuntimeStatus | null>(null);
  const [restoring, setRestoring] = useState(true);
  const [operationState, setOperationState] = useState<OperationState | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const next = await invoke<RuntimeStatus>("mobile_runtime_status");
      setStatus(next);
    } catch (error) {
      console.error("Unable to read mobile runtime status", error);
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
  }, [refreshStatus]);

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
      toast.error("This iPhone has not been bootstrapped by iBridge Desktop.");
      return;
    }
    if (!status.tunnelReachable) {
      toast.error("iBridge Tunnel is not reachable yet.");
      return;
    }

    const path = await openFileDialog({
      multiple: false,
      filters: [{ name: "IPA Files", extensions: ["ipa"] }],
    });
    if (!path) return;

    await startOperation(mobileSideloadOperation, { appPath: path as string });
  }, [loggedInAs, status, startOperation]);

  return (
    <main className="workspace">
      <header className="workspace-header">
        <div className="header-left">
          <div className="title-block">
            <img src={logo} alt="iBridge" className="logo" />
            <div>
              <h1 className="title">iBridge Mobile</h1>
              <p className="subtitle">One-tap sideloading from this iPhone</p>
            </div>
          </div>
        </div>
      </header>

      <div className="workspace-body">
        <section className="workspace-content">
          <section className="workspace-section">
            <p className="section-label">This iPhone</p>
            <GlassCard className="panel">
              <div className="workspace-list">
                <div className="workspace-list-item">
                  Pairing: {status?.pairingReady ? "Ready" : "Missing"}
                </div>
                <div className="workspace-list-item">
                  Tunnel: {status?.tunnelReachable ? "Connected" : "Disconnected"}
                </div>
                <div className="workspace-list-item">
                  Host: {status?.tunnelHost ?? "10.7.0.1"}
                </div>
              </div>
            </GlassCard>
          </section>

          <section className="workspace-section">
            <p className="section-label">Apple ID</p>
            <GlassCard className="panel">
              {restoring && <p>Restoring desktop bootstrap…</p>}
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
