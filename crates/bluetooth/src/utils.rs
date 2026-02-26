use windows::{
    Devices::{
        Bluetooth::{BluetoothConnectionStatus, BluetoothDevice},
        Enumeration::{DeviceInformation, DeviceInformationCollection},
    },
    core::HSTRING,
};

use crate::{
    Device, DeviceConnectionState,
    apple_cp::{self, AppleDeviceExt, AppleDeviceModel},
};

const CONNECTED_AEP_BLUETOOTH_QUERY: &str =
    "System.Devices.Aep.ProtocolId:=\"{e0cbf06c-cd8b-4647-bb8a-263b43f0f974}\" \
    AND System.Devices.Aep.IsConnected:=System.StructuredQueryType.Boolean#True";

#[derive(Debug, Clone)]
pub struct ConnectedDeviceSummary {
    pub id: String,
    pub name: String,
    pub address: Option<u64>,
    pub likely_airpods: bool,
    pub connectable: bool,
}

pub fn get_connected_device_informations() -> windows::core::Result<DeviceInformationCollection> {
    let query = BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    )?;
    let devices = DeviceInformation::FindAllAsyncAqsFilter(&query)?.get()?;

    Ok(devices)
}

pub fn get_connected_device_list() -> Vec<Device> {
    let Ok(aqsfilter) = BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    ) else {
        return vec![];
    };

    let Ok(devices) = DeviceInformation::FindAllAsyncAqsFilter(&aqsfilter) else {
        return vec![];
    };

    let Ok(devices) = devices.get() else {
        return vec![];
    };

    let devices = devices.into_iter().filter_map(|device| {
        let device = Device::try_from(device).ok()?;
        Some(device)
    });

    devices.collect()
}

pub fn find_connected_device_with_vendor_id(vendor_id: u16) -> Option<Device> {
    let devices = get_connected_device_list();
    let device = devices.iter().find(|device| {
        device.get_vendor_id() == Ok(vendor_id)
            && device.get_connection_state() == DeviceConnectionState::Connected
    })?;

    Some(device.clone())
}

pub fn get_connected_device_summaries() -> Vec<ConnectedDeviceSummary> {
    let Ok(devices) = DeviceInformation::FindAllAsyncAqsFilter(&HSTRING::from(
        CONNECTED_AEP_BLUETOOTH_QUERY,
    )) else {
        return vec![];
    };

    let Ok(devices) = devices.get() else {
        return vec![];
    };

    devices
        .into_iter()
        .filter_map(|info| {
            let id = info.Id().ok()?.to_string();
            let name = info.Name().ok()?.to_string();
            let mut summary = ConnectedDeviceSummary {
                id: id.clone(),
                name: name.clone(),
                address: None,
                likely_airpods: looks_like_apple_audio_name(&name),
                connectable: false,
            };

            if let Ok(device) = Device::from_device_id(id) {
                summary.connectable = true;
                summary.address = device.get_address().ok();

                if let Ok(vendor_id) = device.get_vendor_id() {
                    if vendor_id == apple_cp::VENDOR_ID {
                        summary.likely_airpods = true;
                    }
                }

                if !matches!(device.get_device_model(), AppleDeviceModel::Unknown) {
                    summary.likely_airpods = true;
                }
            }

            Some(summary)
        })
        .collect()
}

fn looks_like_apple_audio_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("airpods") || lower.contains("beats")
}
