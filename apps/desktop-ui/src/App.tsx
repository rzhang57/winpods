import { useEffect, useMemo, useRef, useState } from "react";
import {BackendData} from "./vite-env";

type AsciiVersion = {
  id: string;
  label: string;
  file: string;
};

type AsciiManifest = {
  defaultVersion: string;
  versions: AsciiVersion[];
};

function batteryBar(level: number): string {
  const clamped = Math.max(0, Math.min(100, level));
  const filled = Math.round(clamped / 10);
  return `[${"#".repeat(filled)}${".".repeat(10 - filled)}]`;
}

function formatSync(epochMs?: number): string {
  if (!epochMs) return "N/A";
  return new Date(epochMs).toLocaleTimeString();
}

function mapModelToAsciiId(model?: string): string | null {
  if (!model) return null;
  const value = model.toLowerCase();
  if (value.includes("airpodsmax")) return "airpods-max";
  if (value.includes("beatsfitpro")) return "beats-fit-pro";
  if (value.includes("airpodspro2") || value.includes("airpodspro2usbc")) {
    return "airpods-pro-2";
  }
  if (value.includes("airpods3") || value.includes("airpods4") || value.includes("airpods2")) {
    return "airpods-4";
  }
  return null;
}

export default function App() {
  const [theme, setTheme] = useState<"light" | "dark">("dark");
  const [versions, setVersions] = useState<AsciiVersion[]>([]);
  const [model, setModel] = useState<string>("airpods-pro-2");
  const [ascii, setAscii] = useState<string>("");
  const [backendRunning, setBackendRunning] = useState(false);
  const [backendData, setBackendData] = useState<BackendData | null>(null);
  const [deviceSectionWidth, setDeviceSectionWidth] = useState<number>(420);
  const [deviceSectionHeight, setDeviceSectionHeight] = useState<number>(260);
  const deviceSectionRef = useRef<HTMLElement | null>(null);
  const headerRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    if (!deviceSectionRef.current) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      setDeviceSectionWidth(entry.contentRect.width);
      setDeviceSectionHeight(entry.contentRect.height);
    });

    observer.observe(deviceSectionRef.current);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    let mounted = true;

    const poll = async () => {
      if (!window.backend?.status) return;
      const [res, data] = await Promise.all([
        window.backend.status(),
        window.backend.data?.() ?? Promise.resolve(null)
      ]);
      if (!mounted) return;
      setBackendRunning(Boolean(res?.running));
      setBackendData(data);
    };

    poll();
    const id = window.setInterval(poll, 1500);

    return () => {
      mounted = false;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    let mounted = true;

    const loadManifest = async () => {
      const response = await fetch("ascii/manifest.json");
      const manifest = (await response.json()) as AsciiManifest;
      if (!mounted) return;

      setVersions(manifest.versions);
      setModel(manifest.defaultVersion);
    };

    loadManifest().catch(() => {
      setAscii("[ ASCII ART LOAD FAILED ]");
    });

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    const mapped = mapModelToAsciiId(backendData?.device?.model);
    if (mapped && versions.some((v) => v.id === mapped)) {
      setModel(mapped);
    }
  }, [backendData?.device?.model, versions]);

  useEffect(() => {
    if (!model) return;

    const version = versions.find((entry) => entry.id === model);
    if (!version) return;

    fetch(`ascii/${version.file}`)
      .then((response) => response.text())
      .then((text) => setAscii(text))
      .catch(() => setAscii("[ ASCII ART FILE NOT FOUND ]"));
  }, [model, versions]);

  const deviceConnected =
    Boolean(backendData?.device) && backendData?.device?.connectionState === "connected";
  const props = backendData?.properties;

  const metaRows = useMemo(
    () => [
      ["Status", backendRunning ? "Connected" : "Offline"],
      ["Device", deviceConnected ? "Connected" : "Disconnected"],
      ["Name", backendData?.device?.name ?? "N/A"],
      ["Model", backendData?.device?.model ?? "N/A"],
      ["Left Ear In", props ? (props.leftInEar ? "YES" : "NO") : "N/A"],
      ["Right Ear In", props ? (props.rightInEar ? "YES" : "NO") : "N/A"]
    ],
    [backendRunning, deviceConnected, backendData, props]
  );

  const asciiFontSizePx = useMemo(() => {
    const lines = ascii.split("\n");
    const longestLine = lines.reduce((max, line) => Math.max(max, line.length), 1);
    const lineCount = lines.length;

    // Width constraint: approximate monospace glyph width ~= 0.62em
    const usableWidth = Math.max(120, deviceSectionWidth - 40);
    const widthFitted = usableWidth / (longestLine * 0.62);

    // Height constraint: subtract header height and ascii element top + bottom padding (10px each)
    const headerH = headerRef.current?.offsetHeight ?? 35;
    const usableHeight = Math.max(60, deviceSectionHeight - headerH - 20);
    // line-height in CSS is 0.95, so each line occupies fontSize * 0.95 px
    const heightFitted = usableHeight / (lineCount * 0.95);

    return Math.max(4.5, Math.min(12, Math.min(widthFitted, heightFitted)));
  }, [ascii, deviceSectionWidth, deviceSectionHeight]);

  return (
    <main className="app">
      <section className="window-grid">
        <section className="window device-window" ref={deviceSectionRef}>
          <header className="window-header" ref={headerRef}>DEVICE</header>
          <pre
            className="ascii"
            style={{
              fontSize: `${asciiFontSizePx}px`
            }}
          >
            {ascii}
          </pre>
        </section>

        <section className="window meta-window">
          <header className="window-header">DEVICE META</header>
          {metaRows.map(([label, value]) => (
            <div className="row" key={label}>
              <span>{label}</span>
              <span>{value}</span>
            </div>
          ))}
        </section>

        <section className="window battery-window">
          <header className="window-header">BATTERY</header>
          {props ? (
            <>
              <div className="row mono-row">
                <span>Left</span>
                <span>{`${batteryBar(props.leftBattery.level)} ${props.leftBattery.level}%${props.leftBattery.charging ? " CHG" : ""}`}</span>
              </div>
              <div className="row mono-row">
                <span>Right</span>
                <span>{`${batteryBar(props.rightBattery.level)} ${props.rightBattery.level}%${props.rightBattery.charging ? " CHG" : ""}`}</span>
              </div>
              {props.caseBattery && (
                <div className="row mono-row">
                  <span>Case</span>
                  <span>{`${batteryBar(props.caseBattery.level)} ${props.caseBattery.level}%${props.caseBattery.charging ? " CHG" : ""}`}</span>
                </div>
              )}
            </>
          ) : (
            <div className="fallback">No battery data available.</div>
          )}
        </section>

        <section className="window anc-window">
          <header className="window-header">ANC MODE</header>
          <div className="row">
            <span>Current</span>
            <span>N/A (not exposed by backend yet)</span>
          </div>
        </section>

        <section className="window sync-window">
          <header className="window-header">LAST SYNC</header>
          <div className="row">
            <span>Updated</span>
            <span>{formatSync(backendData?.updatedAtEpochMs)}</span>
          </div>
          <div className="row">
            <span>Theme</span>
            <div className="theme-actions">
              <button
                className={theme === "light" ? "button active" : "button"}
                onClick={() => setTheme("light")}
              >
                LIGHT
              </button>
              <button
                className={theme === "dark" ? "button active" : "button"}
                onClick={() => setTheme("dark")}
              >
                DARK
              </button>
            </div>
          </div>
        </section>
      </section>
    </main>
  );
}
