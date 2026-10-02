import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import MobileApp from "./MobileApp";
import { Toaster } from "sonner";
import { StoreProvider } from "./StoreContext";
import { LogProvider } from "./LogContext";
import { ErrorProvider } from "./ErrorContext";
import { DialogProvider } from "./DialogContext";
import "./i18next";
import { PlatformProvider } from "./PlatformContext";

const ua = navigator.userAgent || "";
const platformName = navigator.platform || "";
const isTouchMac = platformName === "MacIntel" && navigator.maxTouchPoints > 1;
const isIOS = /iPhone|iPad|iPod/i.test(ua) || isTouchMac;
const RootApp = isIOS ? MobileApp : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <PlatformProvider>
      <StoreProvider>
        <ErrorProvider>
          <DialogProvider>
            <LogProvider>
              <RootApp />
            </LogProvider>
          </DialogProvider>
        </ErrorProvider>
      </StoreProvider>
    </PlatformProvider>
    <Toaster richColors expand />
  </React.StrictMode>,
);
