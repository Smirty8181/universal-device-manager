use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use windows::core::GUID;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCommunicationStatus, GattDeviceService,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};
use windows::Storage::Streams::DataReader;

const BATTERY_SERVICE: u128 = 0x0000_180F_0000_1000_8000_0080_5F9B_34FB;
const BATTERY_LEVEL: u128 = 0x0000_2A19_0000_1000_8000_0080_5F9B_34FB;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GattServiceInfo {
    pub uuid: String,
    pub characteristics: Vec<String>,
}

fn short_uuid(guid: GUID) -> String {
    let value = guid.to_u128();
    let base = 0x0000_1000_8000_0080_5F9B_34FB_u128;
    let high = value >> 96;
    if high <= 0xFFFF && (value & ((1u128 << 96) - 1)) == base {
        format!("{high:04X}")
    } else {
        format!("{value:032X}")
    }
}

pub fn address_from_instance(instance: &str) -> Option<u64> {
    let upper = instance.to_ascii_uppercase();
    let mut search = upper.as_str();
    while let Some(position) = search.find("DEV_") {
        let start = position + 4;
        let candidate = search.get(start..start + 12)?;
        if candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            return u64::from_str_radix(candidate, 16).ok();
        }
        search = search.get(start..)?;
    }
    None
}

fn open_device(address: u64) -> Result<BluetoothLEDevice, String> {
    let device = BluetoothLEDevice::FromBluetoothAddressAsync(address)
        .map_err(|e| format!("BLE open: {e}"))?
        .get()
        .map_err(|e| format!("BLE open: {e}"))?;
    Ok(device)
}

fn battery_characteristic(device: &BluetoothLEDevice) -> Result<GattCharacteristic, String> {
    let result = device
        .GetGattServicesForUuidAsync(GUID::from_u128(BATTERY_SERVICE))
        .map_err(|e| format!("GATT service request: {e}"))?
        .get()
        .map_err(|e| format!("GATT service request: {e}"))?;
    if result.Status().map_err(|e| e.to_string())? != GattCommunicationStatus::Success {
        return Err("battery service not available".into());
    }
    let services = result.Services().map_err(|e| e.to_string())?;
    let count = services.Size().map_err(|e| e.to_string())?;
    for index in 0..count {
        let service: GattDeviceService = services.GetAt(index).map_err(|e| e.to_string())?;
        let request = service
            .GetCharacteristicsForUuidAsync(GUID::from_u128(BATTERY_LEVEL))
            .map_err(|e| format!("GATT characteristics: {e}"))?
            .get()
            .map_err(|e| format!("GATT characteristics: {e}"))?;
        if request.Status().map_err(|e| e.to_string())? != GattCommunicationStatus::Success {
            continue;
        }
        let characteristics = request.Characteristics().map_err(|e| e.to_string())?;
        if characteristics.Size().map_err(|e| e.to_string())? > 0 {
            return characteristics.GetAt(0).map_err(|e| e.to_string());
        }
    }
    Err("battery level characteristic 0x2A19 not found".into())
}

fn read_battery_inner(address: u64) -> Result<u8, String> {
    let device = open_device(address)?;
    let characteristic = battery_characteristic(&device)?;
    let read = characteristic
        .ReadValueAsync()
        .map_err(|e| format!("GATT read: {e}"))?
        .get()
        .map_err(|e| format!("GATT read: {e}"))?;
    if read.Status().map_err(|e| e.to_string())? != GattCommunicationStatus::Success {
        return Err("GATT read failed".into());
    }
    let buffer = read.Value().map_err(|e| e.to_string())?;
    let reader = DataReader::FromBuffer(&buffer).map_err(|e| e.to_string())?;
    let percent = reader.ReadByte().map_err(|e| e.to_string())?;
    if percent > 100 {
        return Err(format!("BLE battery level {percent} is out of range"));
    }
    Ok(percent)
}

fn inspect_inner(address: u64) -> Result<Vec<GattServiceInfo>, String> {
    let device = open_device(address)?;
    let result = device
        .GetGattServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|e| format!("GATT services: {e}"))?
        .get()
        .map_err(|e| format!("GATT services: {e}"))?;
    if result.Status().map_err(|e| e.to_string())? != GattCommunicationStatus::Success {
        return Err("GATT services unavailable".into());
    }
    let services = result.Services().map_err(|e| e.to_string())?;
    let count = services.Size().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for index in 0..count {
        let service: GattDeviceService = services.GetAt(index).map_err(|e| e.to_string())?;
        let uuid = service.Uuid().map(short_uuid).unwrap_or_default();
        let mut characteristics = Vec::new();
        if let Ok(request) = service.GetCharacteristicsAsync() {
            if let Ok(result) = request.get() {
                if result.Status().ok() == Some(GattCommunicationStatus::Success) {
                    if let Ok(list) = result.Characteristics() {
                        let char_count = list.Size().unwrap_or(0);
                        for char_index in 0..char_count {
                            if let Ok(characteristic) = list.GetAt(char_index) {
                                if let Ok(char_uuid) = characteristic.Uuid() {
                                    characteristics.push(short_uuid(char_uuid));
                                }
                            }
                        }
                    }
                }
            }
        }
        out.push(GattServiceInfo {
            uuid,
            characteristics,
        });
    }
    Ok(out)
}

fn with_timeout<T: Send + 'static>(
    address: u64,
    work: fn(u64) -> Result<T, String>,
    seconds: u64,
) -> Result<T, String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(work(address));
    });
    receiver
        .recv_timeout(Duration::from_secs(seconds))
        .map_err(|_| "Bluetooth GATT timeout".to_string())?
}

pub fn read_battery(address: u64) -> Result<u8, String> {
    with_timeout(address, read_battery_inner, 5)
}

pub fn inspect(address: u64) -> Result<Vec<GattServiceInfo>, String> {
    with_timeout(address, inspect_inner, 6)
}
