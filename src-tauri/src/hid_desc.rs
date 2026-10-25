use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

use hidapi::HidApi;
use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetPreparsedData, HidP_Feature, HidP_GetButtonCaps, HidP_GetCaps,
    HidP_GetUsageValue, HidP_GetValueCaps, HidP_Input, HidP_Output, HIDP_BUTTON_CAPS, HIDP_CAPS,
    HIDP_REPORT_TYPE, HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS, PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HidField {
    pub kind: String,
    pub report_id: u8,
    pub usage_page: u16,
    pub usage: u16,
    pub bit_size: u32,
    pub report_count: u32,
    pub logical_min: i64,
    pub logical_max: i64,
    pub link_collection: u16,
}

#[derive(Clone, Debug)]
pub struct HidCapsInfo {
    pub usage_page: u16,
    pub usage: u16,
    pub input_len: u16,
    pub output_len: u16,
    pub feature_len: u16,
    pub fields: Vec<HidField>,
}

struct Preparsed {
    handle: HANDLE,
    data: PHIDP_PREPARSED_DATA,
}

impl Drop for Preparsed {
    fn drop(&mut self) {
        unsafe {
            let _ = HidD_FreePreparsedData(self.data);
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Preparsed {
    fn open(path: &str) -> Result<Self, String> {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .or_else(|_| unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                0x8000_0000,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        })
        .map_err(|error| error.message().to_string())?;
        let mut data = PHIDP_PREPARSED_DATA::default();
        let ok = unsafe { HidD_GetPreparsedData(handle, &mut data) }.as_bool();
        if !ok || data.0 == 0 {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err("HidD_GetPreparsedData failed".into());
        }
        Ok(Self { handle, data })
    }
}

fn caps_cache() -> &'static Mutex<HashMap<String, HidCapsInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, HidCapsInfo>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn hex_bytes(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn usage_of_value(caps: &HIDP_VALUE_CAPS) -> u16 {
    if caps.IsRange.as_bool() {
        unsafe { caps.Anonymous.Range.UsageMin }
    } else {
        unsafe { caps.Anonymous.NotRange.Usage }
    }
}

fn usage_of_button(caps: &HIDP_BUTTON_CAPS) -> u16 {
    if caps.IsRange.as_bool() {
        unsafe { caps.Anonymous.Range.UsageMin }
    } else {
        unsafe { caps.Anonymous.NotRange.Usage }
    }
}

fn collect_values(
    report_type: HIDP_REPORT_TYPE,
    kind: &str,
    count: u16,
    prep: PHIDP_PREPARSED_DATA,
    fields: &mut Vec<HidField>,
) {
    if count == 0 {
        return;
    }
    let mut caps = vec![HIDP_VALUE_CAPS::default(); count as usize];
    let mut length = count;
    let status = unsafe { HidP_GetValueCaps(report_type, caps.as_mut_ptr(), &mut length, prep) };
    if status.0 != HIDP_STATUS_SUCCESS.0 {
        return;
    }
    for cap in caps.into_iter().take(length as usize) {
        fields.push(HidField {
            kind: kind.to_string(),
            report_id: cap.ReportID,
            usage_page: cap.UsagePage,
            usage: usage_of_value(&cap),
            bit_size: cap.BitSize as u32,
            report_count: cap.ReportCount as u32,
            logical_min: cap.LogicalMin as i64,
            logical_max: cap.LogicalMax as i64,
            link_collection: cap.LinkCollection,
        });
    }
}

fn collect_buttons(
    report_type: HIDP_REPORT_TYPE,
    kind: &str,
    count: u16,
    prep: PHIDP_PREPARSED_DATA,
    fields: &mut Vec<HidField>,
) {
    if count == 0 {
        return;
    }
    let mut caps = vec![HIDP_BUTTON_CAPS::default(); count as usize];
    let mut length = count;
    let status = unsafe { HidP_GetButtonCaps(report_type, caps.as_mut_ptr(), &mut length, prep) };
    if status.0 != HIDP_STATUS_SUCCESS.0 {
        return;
    }
    for cap in caps.into_iter().take(length as usize) {
        fields.push(HidField {
            kind: kind.to_string(),
            report_id: cap.ReportID,
            usage_page: cap.UsagePage,
            usage: usage_of_button(&cap),
            bit_size: 1,
            report_count: cap.ReportCount as u32,
            logical_min: 0,
            logical_max: 1,
            link_collection: cap.LinkCollection,
        });
    }
}

fn read_caps(path: &str) -> Result<HidCapsInfo, String> {
    let prep = Preparsed::open(path)?;
    let mut caps = HIDP_CAPS::default();
    let status = unsafe { HidP_GetCaps(prep.data, &mut caps) };
    if status.0 != HIDP_STATUS_SUCCESS.0 {
        return Err("HidP_GetCaps rejected the preparsed data".into());
    }
    let mut fields = Vec::new();
    collect_buttons(
        HidP_Input,
        "input",
        caps.NumberInputButtonCaps,
        prep.data,
        &mut fields,
    );
    collect_values(
        HidP_Input,
        "input",
        caps.NumberInputValueCaps,
        prep.data,
        &mut fields,
    );
    collect_buttons(
        HidP_Output,
        "output",
        caps.NumberOutputButtonCaps,
        prep.data,
        &mut fields,
    );
    collect_values(
        HidP_Output,
        "output",
        caps.NumberOutputValueCaps,
        prep.data,
        &mut fields,
    );
    collect_buttons(
        HidP_Feature,
        "feature",
        caps.NumberFeatureButtonCaps,
        prep.data,
        &mut fields,
    );
    collect_values(
        HidP_Feature,
        "feature",
        caps.NumberFeatureValueCaps,
        prep.data,
        &mut fields,
    );
    Ok(HidCapsInfo {
        usage_page: caps.UsagePage,
        usage: caps.Usage,
        input_len: caps.InputReportByteLength,
        output_len: caps.OutputReportByteLength,
        feature_len: caps.FeatureReportByteLength,
        fields,
    })
}

pub fn caps_for_path(path: &str) -> Result<HidCapsInfo, String> {
    if let Some(cached) = caps_cache().lock().unwrap().get(path) {
        return Ok(cached.clone());
    }
    let info = read_caps(path)?;
    let mut cache = caps_cache().lock().unwrap();
    if cache.len() > 512 {
        cache.clear();
    }
    cache.insert(path.to_string(), info.clone());
    Ok(info)
}

pub fn caps_text(caps: &HidCapsInfo) -> String {
    format!(
        "HID caps · 0x{:04X}:0x{:02X}\nReports: input {} B · output {} B · feature {} B\nFields:\n{}",
        caps.usage_page,
        caps.usage,
        caps.input_len,
        caps.output_len,
        caps.feature_len,
        usage_summary(&caps.fields)
    )
}

pub fn battery_field(caps: &HidCapsInfo) -> Option<HidField> {
    let mut candidates: Vec<HidField> = caps
        .fields
        .iter()
        .filter(|field| {
            field.usage_page == 0x06
                && field.usage == 0x20
                && matches!(field.kind.as_str(), "input" | "feature")
                && field.bit_size >= 8
                && field.logical_max > 0
                && field.logical_max <= 100
        })
        .cloned()
        .collect();
    candidates.sort_by_key(|field| match field.kind.as_str() {
        "feature" => 0,
        _ => 1,
    });
    candidates.into_iter().next()
}

pub fn battery_field_for_path(path: &str) -> Result<HidField, String> {
    let caps = caps_for_path(path)?;
    battery_field(&caps).ok_or_else(|| "no HID Battery Strength usage in the collection".into())
}

fn decode_usage(
    prep: PHIDP_PREPARSED_DATA,
    report_type: HIDP_REPORT_TYPE,
    field: &HidField,
    report: &[u8],
) -> Result<u8, String> {
    let mut value = 0u32;
    let status = unsafe {
        HidP_GetUsageValue(
            report_type,
            field.usage_page,
            field.link_collection,
            field.usage,
            &mut value,
            prep,
            report,
        )
    };
    if status.0 != HIDP_STATUS_SUCCESS.0 {
        return Err(format!("HidP_GetUsageValue 0x{:08X}", status.0 as u32));
    }
    let value = value as i64;
    if value < field.logical_min || value > field.logical_max {
        return Err("usage value is outside the logical range".into());
    }
    if field.logical_max > 100 {
        return Err("usage value is not a percentage".into());
    }
    Ok(value.clamp(0, 100) as u8)
}

pub fn read_field(path: &str, field: &HidField) -> Result<u8, String> {
    let prep = Preparsed::open(path)?;
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    let handle = api.open_path(&cpath).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 512];

    if field.kind == "feature" {
        buf[0] = field.report_id;
        let read = handle
            .get_feature_report(&mut buf)
            .map_err(|e| format!("feature read: {e}"))?;
        if read == 0 {
            return Err("empty feature report".into());
        }
        return decode_usage(prep.data, HidP_Feature, field, &buf[..read]);
    }

    for _ in 0..8 {
        if let Ok(read) = handle.read_timeout(&mut buf, 120) {
            if read > 0 {
                if let Ok(value) = decode_usage(prep.data, HidP_Input, field, &buf[..read]) {
                    return Ok(value);
                }
            }
        }
    }
    Err("no input report with a valid battery usage".into())
}

pub fn usage_summary(fields: &[HidField]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    for field in fields {
        seen.insert(format!(
            "{:02X}:{:02X} {} id={} {}/{}",
            field.usage_page,
            field.usage,
            field.kind,
            field.report_id,
            field.bit_size,
            field.logical_max
        ));
    }
    if seen.is_empty() {
        "—".into()
    } else {
        seen.into_iter().collect::<Vec<_>>().join("\n")
    }
}
