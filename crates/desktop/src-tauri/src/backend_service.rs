use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use bluetooth::{
    AdapterState, AdapterWatcher, AdvertisementReceivedData, AdvertisementWatcher, Device,
    DeviceConnectionState,
    apple_cp::{self, AppleDeviceExt, AppleDeviceModel},
};
use media::GlobalMediaController;
use serde::{Deserialize, Serialize};
use tracing::{Level, debug, error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Battery {
    level: u8,
    charging: bool,
}

impl Battery {
    fn new(level: u8, charging: bool) -> Self {
        Self { level, charging }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceProperties {
    model: AppleDeviceModel,
    left_battery: Battery,
    right_battery: Battery,
    case_battery: Option<Battery>,
    left_in_ear: bool,
    right_in_ear: bool,
}

impl DeviceProperties {
    fn from_advertisement(
        data: &AdvertisementReceivedData,
        protocol: &apple_cp::ProximityPairingMessage,
    ) -> Option<Self> {
        if data.manufacturer_data_map.get(&apple_cp::VENDOR_ID).is_none() {
            return None;
        }

        let model = protocol.get_model();
        let right_battery = Battery::new(
            protocol.get_right_battery().unwrap_or(0) * 10,
            protocol.is_right_charging(),
        );
        let left_battery = Battery::new(
            protocol.get_left_battery().unwrap_or(0) * 10,
            protocol.is_left_charging(),
        );
        let case_battery = protocol
            .get_case_battery()
            .map(|value| Battery::new(value * 10, protocol.is_case_charging()));

        Some(Self {
            model,
            right_battery,
            left_battery,
            case_battery,
            left_in_ear: protocol.is_left_in_ear(),
            right_in_ear: protocol.is_right_in_ear(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsState {
    auto_start: bool,
    auto_update: bool,
    low_battery_threshold: u8,
    ear_detection: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            auto_start: true,
            auto_update: true,
            low_battery_threshold: 20,
            ear_detection: true,
        }
    }
}

impl SettingsState {
    fn load() -> Self {
        let Ok(contents) = fs::read_to_string(settings_path()) else {
            return Self::default();
        };

        serde_json::from_str(&contents).unwrap_or_default()
    }
}

#[derive(Default)]
struct EarDetectionRuntime {
    paused: bool,
    media_controller: GlobalMediaController,
}

struct ServiceState {
    adapter_state: AdapterState,
    current_device: Option<Device>,
    current_properties: Option<DeviceProperties>,
    settings: SettingsState,
    low_battery_sent: bool,
    ear_runtime: EarDetectionRuntime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceSnapshot {
    device_id: String,
    name: String,
    address: u64,
    model: AppleDeviceModel,
    connection_state: DeviceConnectionState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendSnapshot {
    updated_at_epoch_ms: u128,
    adapter_state: AdapterState,
    device: Option<DeviceSnapshot>,
    properties: Option<DeviceProperties>,
    settings: SettingsState,
    low_battery_alerted: bool,
}

impl ServiceState {
    fn new() -> Self {
        Self {
            adapter_state: AdapterState::Off,
            current_device: None,
            current_properties: None,
            settings: SettingsState::load(),
            low_battery_sent: false,
            ear_runtime: EarDetectionRuntime::default(),
        }
    }
}

pub fn run() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    info!("starting winpods backend service");

    let shared = Arc::new(Mutex::new(ServiceState::new()));
    let mut adapter_watcher = AdapterWatcher::new();
    let adv_watcher = Arc::new(
        AdvertisementWatcher::new().map_err(|err| anyhow::anyhow!("adv watcher init: {err:?}"))?,
    );

    install_adapter_callbacks(&mut adapter_watcher, adv_watcher.clone(), shared.clone());
    install_advertisement_callbacks(adv_watcher.clone(), shared.clone());

    {
        let mut state = shared.lock().expect("state poisoned");
        if let Some(device) = bluetooth::find_connected_device_with_vendor_id(apple_cp::VENDOR_ID) {
            attach_device(device, shared.clone(), &mut state);
            info!("autodetected connected Apple audio device at startup");
        }

        state.adapter_state = adapter_watcher.state();
        write_snapshot(&state);
    }

    adapter_watcher.start();

    if matches!(adapter_watcher.state(), AdapterState::On) {
        let _ = adv_watcher.start();
    }

    // Background service loop: keep process alive and periodically re-autodetect.
    let mut last_autodetect = Instant::now();
    loop {
        if last_autodetect.elapsed() >= Duration::from_secs(2) {
            try_autodetect(shared.clone(), "periodic");
            last_autodetect = Instant::now();
        }

        thread::sleep(Duration::from_millis(150));
    }
}

fn install_adapter_callbacks(
    adapter_watcher: &mut AdapterWatcher,
    adv_watcher: Arc<AdvertisementWatcher>,
    shared: Arc<Mutex<ServiceState>>,
) {
    adapter_watcher.on_state_changed(move |new_state| {
        if matches!(new_state, AdapterState::On) {
            let _ = adv_watcher.start();
        } else {
            let _ = adv_watcher.stop();
        }

        let mut state = shared.lock().expect("state poisoned");
        state.adapter_state = new_state;
        info!("bluetooth adapter state changed: {new_state:?}");
        write_snapshot(&state);
    });
}

fn install_advertisement_callbacks(
    adv_watcher: Arc<AdvertisementWatcher>,
    shared: Arc<Mutex<ServiceState>>,
) {
    let state_for_received = shared.clone();
    adv_watcher.on_received(move |data| {
        process_advertisement(data, &state_for_received);
    });

    adv_watcher.on_stopped(move || {
        debug!("advertisement watcher stopped");
    });
}

fn attach_device(device: Device, shared: Arc<Mutex<ServiceState>>, state: &mut ServiceState) {
    let state_for_name = shared.clone();
    device.on_name_changed(move |name| {
        let state = state_for_name.lock().expect("state poisoned");
        info!("device name updated: {name}");
        write_snapshot(&state);
    });

    let state_for_connection = shared.clone();
    device.on_connection_changed(move |connection_state| {
        let mut state = state_for_connection.lock().expect("state poisoned");
        info!("connection state changed: {connection_state:?}");
        if matches!(connection_state, DeviceConnectionState::Disconnected) {
            state.current_device = None;
            state.current_properties = None;
            state.ear_runtime.paused = false;
            state.ear_runtime.media_controller.reset();
        }
        write_snapshot(&state);
    });

    state.current_device = Some(device);
    state.current_properties = None;
    state.low_battery_sent = false;
    write_snapshot(state);
}

fn try_autodetect(shared: Arc<Mutex<ServiceState>>, reason: &str) {
    let should_search = {
        let state = shared.lock().expect("state poisoned");
        if let Some(device) = &state.current_device {
            !device.is_connected()
        } else {
            true
        }
    };

    if !should_search {
        return;
    }

    if let Some(device) = bluetooth::find_connected_device_with_vendor_id(apple_cp::VENDOR_ID) {
        let mut state = shared.lock().expect("state poisoned");
        attach_device(device, shared.clone(), &mut state);
        info!("autodetected Apple device ({reason})");
        write_snapshot(&state);
    }
}

fn process_advertisement(data: &AdvertisementReceivedData, shared: &Arc<Mutex<ServiceState>>) {
    let Some(apple_data) = data.manufacturer_data_map.get(&apple_cp::VENDOR_ID) else {
        return;
    };

    let Some(protocol) = apple_cp::proximity_pairing_message_from_bytes(apple_data) else {
        return;
    };

    let (device, settings) = {
        let state = shared.lock().expect("state poisoned");
        (state.current_device.clone(), state.settings.clone())
    };

    let Some(device) = device else {
        return;
    };

    if !device.is_connected() {
        return;
    }

    let Some(properties) = DeviceProperties::from_advertisement(data, &protocol) else {
        return;
    };

    if device.get_device_model() != properties.model {
        return;
    }

    let mut state = shared.lock().expect("state poisoned");
    state.current_properties = Some(properties.clone());

    if settings.ear_detection {
        let both_in_ear = properties.left_in_ear && properties.right_in_ear;
        if both_in_ear && state.ear_runtime.paused {
            if state.ear_runtime.media_controller.resume().is_ok() {
                state.ear_runtime.paused = false;
            }
        } else if !both_in_ear && !state.ear_runtime.paused {
            if state.ear_runtime.media_controller.pause().is_ok() {
                state.ear_runtime.paused = true;
            }
        }
    }

    let threshold = settings.low_battery_threshold;
    if threshold == 0 {
        return;
    }

    let low_left = !properties.left_battery.charging && properties.left_battery.level <= threshold;
    let low_right = !properties.right_battery.charging && properties.right_battery.level <= threshold;

    if (low_left || low_right) && !state.low_battery_sent {
        state.low_battery_sent = true;
        info!("low battery alert: at least one bud <= {}%", threshold);
    } else if !low_left && !low_right {
        state.low_battery_sent = false;
    }

    write_snapshot(&state);
}

fn settings_path() -> PathBuf {
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("winpods");
        dir.push("settings.json");
        dir
    } else {
        error!("failed to resolve config dir; falling back to local settings.json");
        PathBuf::from("settings.json")
    }
}

fn state_path() -> PathBuf {
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("winpods");
        dir.push("state.json");
        dir
    } else {
        PathBuf::from("state.json")
    }
}

fn write_snapshot(state: &ServiceState) {
    let snapshot = BackendSnapshot {
        updated_at_epoch_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        adapter_state: state.adapter_state,
        device: state.current_device.as_ref().map(|device| DeviceSnapshot {
            device_id: device.get_device_id().unwrap_or_default(),
            name: device.get_name().unwrap_or_else(|_| "Unknown".to_string()),
            address: device.get_address().unwrap_or(0),
            model: device.get_device_model(),
            connection_state: device.get_connection_state(),
        }),
        properties: state.current_properties.clone(),
        settings: state.settings.clone(),
        low_battery_alerted: state.low_battery_sent,
    };

    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(serialized) = serde_json::to_vec_pretty(&snapshot) {
        let _ = fs::write(path, serialized);
    }
}
