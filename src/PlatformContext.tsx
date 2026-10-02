import React, { createContext, useContext, useEffect, useState } from "react";
import "./DialogContext.css";

export type IBridgePlatform = "windows" | "mac" | "linux" | "ios";

export const PlatformContext = createContext<{
  platform: IBridgePlatform;
}>({ platform: "windows" });

export const PlatformProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [platform, setPlatform] = useState<IBridgePlatform>("windows");

  useEffect(() => {
    if (typeof navigator === "undefined") return;
    const ua = navigator.userAgent || "";
    const platformName = navigator.platform || "";
    const touchMac = platformName === "MacIntel" && navigator.maxTouchPoints > 1;

    if (/iPhone|iPad|iPod/i.test(ua) || touchMac) {
      setPlatform("ios");
    } else if (ua.includes("Mac")) {
      setPlatform("mac");
    } else if (ua.includes("Win")) {
      setPlatform("windows");
    } else if (ua.includes("Linux")) {
      setPlatform("linux");
    }
  }, []);

  return (
    <PlatformContext.Provider value={{ platform }}>
      {children}
    </PlatformContext.Provider>
  );
};

export const usePlatform = () => {
  return useContext(PlatformContext);
};
