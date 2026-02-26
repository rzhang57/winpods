export type BackendStatus = { running: boolean };
export type BackendData = {
  updatedAtEpochMs: number;
  adapterState: "on" | "off";
  device: null | {
    deviceId: string;
    name: string;
    address: number;
    model: string;
    connectionState: "connected" | "disconnected";
  };
  properties: null | {
    model: string;
    leftBattery: { level: number; charging: boolean };
    rightBattery: { level: number; charging: boolean };
    caseBattery?: { level: number; charging: boolean } | null;
    leftInEar: boolean;
    rightInEar: boolean;
  };
  settings: {
    autoStart: boolean;
    autoUpdate: boolean;
    lowBatteryThreshold: number;
    earDetection: boolean;
  };
  lowBatteryAlerted: boolean;
};

declare global {
  interface Window {
    backend: {
      status: () => Promise<BackendStatus>;
      restart: () => Promise<BackendStatus>;
      data: () => Promise<BackendData | null>;
    };
  }
}

export {};
