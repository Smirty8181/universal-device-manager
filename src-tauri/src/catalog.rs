use std::ffi::c_void;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HANDLE, HWND};
use windows::Win32::Networking::WinHttp::*;
use windows::Win32::Security::WinTrust::*;

use crate::model::{AutoDownloadItem, AutoDownloadReport, CatalogPackage, CatalogUpdate};
use crate::store::AppState;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) UniversalDeviceManager/0.1";
const CATALOG: &str = "https://www.catalog.update.microsoft.com";
const MAX_PAGE: u64 = 8 * 1024 * 1024;
const MAX_PACKAGE: u64 = 2 * 1024 * 1024 * 1024;
const AUTO_MAX_PACKAGE: u64 = 250 * 1024 * 1024;
const AUTO_MAX_PACKAGES: usize = 3;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// The generated windows crate wrapper passes the header length as a character count, which
// WinHTTP rejects for POST. Calling through -1 lets the API measure the wide string itself.
#[link(name = "winhttp")]
extern "system" {
    #[link_name = "WinHttpSendRequest"]
    fn send_request(
        request: *mut c_void,
        headers: PCWSTR,
        headers_len: u32,
        optional: *const c_void,
        optional_len: u32,
        total_len: u32,
        context: usize,
    ) -> windows::Win32::Foundation::BOOL;
}

// Codes where Windows itself reports a driver problem; a driver package could actually help.
// Code 24 (device not present) is deliberately excluded: downloading for absent hardware is noise.
const AUTO_PROBLEM_CODES: &[u32] = &[1, 3, 10, 18, 28, 31, 32, 37, 43, 52];

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn last_error(stage: &str) -> String {
    format!("{stage} failed (Win32 error {})", unsafe {
        GetLastError().0
    })
}

struct HttpUrl {
    host: String,
    port: u16,
    path: String,
}

fn split_url(url: &str) -> Result<HttpUrl, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| format!("only https links are allowed: {url}"))?;
    let (host_port, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if host_port.is_empty() {
        return Err("link has no host".into());
    }
    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>().map_err(|_| "bad port".to_string())?,
        ),
        None => (host_port.to_string(), 443),
    };
    Ok(HttpUrl {
        host,
        port,
        path: path.to_string(),
    })
}

struct Response {
    status: u32,
    location: Option<String>,
    body: Vec<u8>,
}

fn read_header(request: *mut c_void, query: u32) -> Option<String> {
    let mut buffer = [0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    unsafe {
        WinHttpQueryHeaders(
            request,
            query,
            PCWSTR::null(),
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut size,
            std::ptr::null_mut(),
        )
        .ok()?;
    }
    let len = (size as usize / 2).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..len]);
    let text = text.trim_matches(char::from(0)).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

unsafe fn request_once(
    method: &str,
    url: &str,
    headers: Option<&str>,
    body: Option<&[u8]>,
) -> Result<Response, String> {
    let parsed = split_url(url)?;
    let session = WinHttpOpen(
        PCWSTR(wide(USER_AGENT).as_ptr()),
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        PCWSTR::null(),
        PCWSTR::null(),
        0,
    );
    if session.is_null() {
        return Err(last_error("WinHttpOpen"));
    }
    let _ = WinHttpSetTimeouts(session, 10_000, 15_000, 30_000, 90_000);
    let result = (|| {
        let connect = WinHttpConnect(session, PCWSTR(wide(&parsed.host).as_ptr()), parsed.port, 0);
        if connect.is_null() {
            return Err(last_error("WinHttpConnect"));
        }
        let verb = wide(method);
        let path = wide(&parsed.path);
        let request = WinHttpOpenRequest(
            connect,
            PCWSTR(verb.as_ptr()),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        );
        if request.is_null() {
            let error = last_error("WinHttpOpenRequest");
            let _ = WinHttpCloseHandle(connect);
            return Err(error);
        }
        let header_wide = headers.map(wide);
        let send = send_request(
            request,
            header_wide
                .as_ref()
                .map_or(PCWSTR::null(), |h| PCWSTR(h.as_ptr())),
            header_wide.as_ref().map_or(0, |_| u32::MAX),
            body.map_or(std::ptr::null(), |b| b.as_ptr() as *const c_void),
            body.map_or(0, |b| b.len() as u32),
            body.map_or(0, |b| b.len() as u32),
            0,
        );
        if send.0 == 0 {
            let error = last_error("WinHttpSendRequest");
            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connect);
            return Err(error);
        }
        if let Err(error) = WinHttpReceiveResponse(request, std::ptr::null_mut()) {
            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connect);
            return Err(format!("WinHttpReceiveResponse: {error}"));
        }
        let mut status = 0u32;
        let mut status_len = std::mem::size_of::<u32>() as u32;
        let _ = WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut status as *mut u32 as *mut c_void),
            &mut status_len,
            std::ptr::null_mut(),
        );
        let location = if (300..400).contains(&status) {
            read_header(request, WINHTTP_QUERY_LOCATION)
        } else {
            None
        };
        let mut body_out = Vec::new();
        let mut chunk = vec![0u8; 65536];
        loop {
            let mut read = 0u32;
            match WinHttpReadData(
                request,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as u32,
                &mut read,
            ) {
                Ok(()) if read > 0 => {
                    body_out.extend_from_slice(&chunk[..read as usize]);
                    if body_out.len() as u64 > MAX_PAGE {
                        let _ = WinHttpCloseHandle(request);
                        let _ = WinHttpCloseHandle(connect);
                        return Err("response is too large".into());
                    }
                }
                Ok(()) => break,
                Err(error) => {
                    let _ = WinHttpCloseHandle(request);
                    let _ = WinHttpCloseHandle(connect);
                    return Err(format!("WinHttpReadData: {error}"));
                }
            }
        }
        let _ = WinHttpCloseHandle(request);
        let _ = WinHttpCloseHandle(connect);
        Ok(Response {
            status,
            location,
            body: body_out,
        })
    })();
    let _ = WinHttpCloseHandle(session);
    result
}

fn http(
    method: &str,
    url: &str,
    headers: Option<&str>,
    body: Option<&[u8]>,
) -> Result<Response, String> {
    let mut current = url.to_string();
    for _ in 0..5 {
        let response = unsafe { request_once(method, &current, headers, body)? };
        if (300..400).contains(&response.status) {
            match response.location {
                Some(next) => {
                    current = if next.starts_with("https://") {
                        next
                    } else if next.starts_with('/') {
                        let parsed = split_url(&current)?;
                        format!("https://{}{}", parsed.host, next)
                    } else {
                        return Err(format!("unexpected redirect: {next}"));
                    };
                    continue;
                }
                None => {
                    return Err(format!(
                        "HTTP {} without a redirect target",
                        response.status
                    ))
                }
            }
        }
        if !(200..300).contains(&response.status) {
            return Err(format!(
                "HTTP {} from {}",
                response.status,
                split_url(&current)?.host
            ));
        }
        return Ok(response);
    }
    Err("too many redirects".into())
}

fn http_text(url: &str) -> Result<String, String> {
    let response = http("GET", url, None, None)?;
    Ok(String::from_utf8_lossy(&response.body).to_string())
}

fn unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    unescape(&out.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn cell_text(block: &str, marker: &str) -> String {
    let Some(start) = block.find(marker) else {
        return String::new();
    };
    let Some(open) = block[start..].find('>') else {
        return String::new();
    };
    let rest = &block[start + open + 1..];
    let end = rest.find("</td>").unwrap_or(rest.len());
    strip_tags(&rest[..end])
}

fn extract_title(block: &str) -> String {
    let Some(start) = block.find("contentTextItemSpacerNoBreakLink\">") else {
        return String::new();
    };
    let rest = &block[start + "contentTextItemSpacerNoBreakLink\">".len()..];
    let end = rest.find("</a>").unwrap_or(rest.len());
    strip_tags(&rest[..end])
}

fn size_of_block(block: &str, guid: &str) -> (u64, String) {
    let marker = format!("{guid}_originalSize\">");
    let bytes = block
        .find(&marker)
        .and_then(|at| block[at + marker.len()..].split('<').next())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let text_marker = format!("{guid}_size\">");
    let text = block
        .find(&text_marker)
        .and_then(|at| block[at + text_marker.len()..].split('<').next())
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    (bytes, text)
}

fn parse_search(html: &str, limit: usize) -> Vec<CatalogUpdate> {
    let mut out: Vec<CatalogUpdate> = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = html[cursor..].find("<tr id=\"") {
        let row_start = cursor + rel;
        let id_start = row_start + "<tr id=\"".len();
        let Some(id_end_rel) = html[id_start..].find('"') else {
            break;
        };
        let id_end = id_start + id_end_rel;
        let row_id = &html[id_start..id_end];
        let Some(guid) = row_id.split("_R").next() else {
            cursor = id_end;
            continue;
        };
        let guid = guid.trim();
        let valid = guid.len() == 36
            && guid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && row_id.contains("_R");
        if !valid {
            cursor = id_end;
            continue;
        }
        let end = html[id_end..]
            .find("</tr>")
            .map(|at| id_end + at + "</tr>".len())
            .unwrap_or(html.len());
        let block = &html[row_start..end];
        let title = extract_title(block);
        if !title.is_empty() && !out.iter().any(|u| u.update_id == guid) {
            let (size_bytes, size_text) = size_of_block(block, guid);
            out.push(CatalogUpdate {
                update_id: guid.to_string(),
                title,
                products: cell_text(block, &format!("{guid}_C2_R")),
                classification: cell_text(block, &format!("{guid}_C3_R")),
                date: cell_text(block, &format!("{guid}_C4_R")),
                version: cell_text(block, &format!("{guid}_C5_R")),
                size_bytes,
                size_text,
                url: format!("{CATALOG}/ScopedViewInline.aspx?updateid={guid}"),
            });
            if out.len() >= limit {
                break;
            }
        }
        cursor = end;
    }
    out
}

fn url_encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

pub fn search(hardware_id: &str) -> Result<Vec<CatalogUpdate>, String> {
    let query = hardware_id.trim();
    if query.len() < 4 {
        return Err("hardware id is too short".into());
    }
    let url = format!("{CATALOG}/Search.aspx?q={}", url_encode(query));
    let page = http_text(&url)?;
    let updates = parse_search(&page, 24);
    crate::store::log_line(&format!(
        "catalog search {query}: {} updates",
        updates.len()
    ));
    Ok(updates)
}

fn dialog_urls(update_id: &str) -> Result<Vec<String>, String> {
    let payload = format!(
        "[{{\"size\":0,\"languages\":\"\",\"uidInfo\":\"{id}\",\"updateID\":\"{id}\"}}]",
        id = update_id
    );
    let body = format!(
        "updateIDs={}&updateIDsBlockedForImport=&wsusApiPresent=&contentImport=&sku=&serverName=&ssl=&portNumber=&version=",
        url_encode(&payload)
    );
    let response = http(
        "POST",
        &format!("{CATALOG}/DownloadDialog.aspx"),
        Some("Content-Type: application/x-www-form-urlencoded\r\n"),
        Some(body.as_bytes()),
    )?;
    let page = String::from_utf8_lossy(&response.body);
    let mut urls: Vec<String> = Vec::new();
    for piece in page.split("files[").skip(1) {
        let Some(at) = piece.find(".url = '") else {
            continue;
        };
        let rest = &piece[at + ".url = '".len()..];
        let Some(end) = rest.find('\'') else {
            continue;
        };
        let url = rest[..end].trim().to_string();
        if url.starts_with("https://") && !urls.contains(&url) {
            urls.push(url);
        }
    }
    if urls.is_empty() {
        return Err("Microsoft catalog returned no download links".into());
    }
    Ok(urls)
}

fn is_guid(text: &str) -> bool {
    text.len() == 36
        && text.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && text.matches('-').count() == 4
}

fn trusted_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    ["windowsupdate.com", "windows.com", "microsoft.com"]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

fn authenticode_ok(path: &Path) -> bool {
    let target = wide(&path.to_string_lossy());
    unsafe {
        let mut file = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(target.as_ptr()),
            hFile: HANDLE::default(),
            pgKnownSubject: std::ptr::null_mut(),
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
            dwStateAction: WTD_STATEACTION_VERIFY,
            ..Default::default()
        };
        let mut action: GUID = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = WinVerifyTrust(
            HWND::default(),
            &mut action,
            &mut data as *mut _ as *mut c_void,
        );
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = WinVerifyTrust(
            HWND::default(),
            &mut action,
            &mut data as *mut _ as *mut c_void,
        );
        status == 0
    }
}

fn download_to(url: &str, target: &Path, cap: u64) -> Result<u64, String> {
    let parsed = split_url(url)?;
    if !trusted_host(&parsed.host) {
        return Err(format!(
            "refusing to download from untrusted host: {}",
            parsed.host
        ));
    }
    unsafe {
        let session = WinHttpOpen(
            PCWSTR(wide(USER_AGENT).as_ptr()),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            return Err(last_error("WinHttpOpen"));
        }
        let _ = WinHttpSetTimeouts(session, 10_000, 15_000, 30_000, 120_000);
        let connect = WinHttpConnect(session, PCWSTR(wide(&parsed.host).as_ptr()), parsed.port, 0);
        if connect.is_null() {
            let error = last_error("WinHttpConnect");
            let _ = WinHttpCloseHandle(session);
            return Err(error);
        }
        let request = WinHttpOpenRequest(
            connect,
            PCWSTR(wide("GET").as_ptr()),
            PCWSTR(wide(&parsed.path).as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        );
        if request.is_null() {
            let error = last_error("WinHttpOpenRequest");
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);
            return Err(error);
        }
        let fail = |message: String| -> String {
            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);
            message
        };
        if let Err(error) = WinHttpSendRequest(request, None, None, 0, 0, 0) {
            return Err(fail(format!("WinHttpSendRequest: {error}")));
        }
        if let Err(error) = WinHttpReceiveResponse(request, std::ptr::null_mut()) {
            return Err(fail(format!("WinHttpReceiveResponse: {error}")));
        }
        let mut status = 0u32;
        let mut status_len = std::mem::size_of::<u32>() as u32;
        let _ = WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut status as *mut u32 as *mut c_void),
            &mut status_len,
            std::ptr::null_mut(),
        );
        if !(200..300).contains(&status) {
            let location = read_header(request, WINHTTP_QUERY_LOCATION);
            let message = match location {
                Some(next) => format!("redirect: {next}"),
                None => format!("HTTP {status} while downloading the package"),
            };
            return Err(fail(message));
        }
        let mut file = fs::File::create(target)
            .map_err(|e| fail(format!("create {}: {e}", target.display())))?;
        let mut total = 0u64;
        let mut chunk = vec![0u8; 65536];
        loop {
            let mut read = 0u32;
            match WinHttpReadData(
                request,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as u32,
                &mut read,
            ) {
                Ok(()) if read > 0 => {
                    total += read as u64;
                    if total > cap {
                        drop(file);
                        let _ = fs::remove_file(target);
                        return Err(fail(format!(
                            "package is larger than {} MB",
                            cap / 1024 / 1024
                        )));
                    }
                    if let Err(error) = file.write_all(&chunk[..read as usize]) {
                        drop(file);
                        return Err(fail(format!("write {}: {error}", target.display())));
                    }
                }
                Ok(()) => break,
                Err(error) => {
                    drop(file);
                    let _ = fs::remove_file(target);
                    return Err(fail(format!("WinHttpReadData: {error}")));
                }
            }
        }
        let _ = WinHttpCloseHandle(request);
        let _ = WinHttpCloseHandle(connect);
        let _ = WinHttpCloseHandle(session);
        if total == 0 {
            let _ = fs::remove_file(target);
            return Err("download returned an empty file".into());
        }
        Ok(total)
    }
}

fn expand_exe() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(|root| Path::new(&root).join("System32").join("expand.exe"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("expand.exe"))
}

fn collect_infs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_infs(&path));
        } else if path
            .extension()
            .map(|e| e.to_string_lossy().eq_ignore_ascii_case("inf"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    out
}

fn meta_path(dir: &Path) -> PathBuf {
    dir.join("meta.json")
}

fn load_existing(dir: &Path, update_id: &str) -> Option<CatalogPackage> {
    let text = fs::read_to_string(meta_path(dir)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    if value.get("updateId")?.as_str()? != update_id {
        return None;
    }
    let files: Vec<String> = value
        .get("files")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let infs: Vec<String> = value
        .get("infs")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    if infs.is_empty() || !infs.iter().all(|p| Path::new(p).is_file()) {
        return None;
    }
    Some(CatalogPackage {
        update_id: update_id.to_string(),
        title: value.get("title")?.as_str()?.to_string(),
        dir: dir.to_string_lossy().to_string(),
        files,
        signature: value.get("signature")?.as_str()?.to_string(),
        infs,
    })
}

pub fn download(data_dir: &Path, update_id: &str, title: &str) -> Result<CatalogPackage, String> {
    if !is_guid(update_id) {
        return Err("invalid update id".into());
    }
    let root = data_dir.join("drivers").join("catalog");
    let dir = root.join(update_id);
    if let Some(existing) = load_existing(&dir, update_id) {
        return Ok(existing);
    }
    crate::store::log_line(&format!("catalog download started: {update_id}"));
    let urls = dialog_urls(update_id)?;
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    fs::create_dir_all(&dir).map_err(|e| format!("create package dir: {e}"))?;
    let mut files = Vec::new();
    let mut total = 0u64;
    for url in &urls {
        let name = url
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty() && name.len() < 120)
            .ok_or("bad package file name")?;
        let clean: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || ".-_".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if clean.chars().all(|c| c == '.') {
            return Err("bad package file name".into());
        }
        let target = dir.join(&clean);
        let bytes = download_to(url, &target, MAX_PACKAGE - total)?;
        total += bytes;
        if !authenticode_ok(&target) {
            let _ = fs::remove_file(&target);
            let _ = fs::remove_dir_all(&dir);
            crate::store::log_line(&format!("catalog package rejected (signature): {url}"));
            return Err("the driver package signature is not valid; the file was deleted".into());
        }
        files.push(target.to_string_lossy().to_string());
    }
    let mut infs: Vec<String> = Vec::new();
    let mut signer = "unknown".to_string();
    for file in &files {
        if !file.to_ascii_lowercase().ends_with(".cab") {
            continue;
        }
        let output = Command::new(expand_exe())
            .creation_flags(CREATE_NO_WINDOW)
            .arg(file)
            .arg("-F:*")
            .arg(&dir)
            .output()
            .map_err(|e| format!("expand.exe: {e}"))?;
        if !output.status.success() {
            let _ = fs::remove_dir_all(&dir);
            return Err("expand.exe could not unpack the driver package".into());
        }
    }
    for inf in collect_infs(&dir) {
        let signature = crate::drivers::signer_of_path(&inf.to_string_lossy());
        if signature == "windows" || signature == "publisher" {
            signer = signature;
            infs.push(inf.to_string_lossy().to_string());
        }
    }
    if infs.is_empty() {
        let _ = fs::remove_dir_all(&dir);
        let has_installer = files.iter().any(|f| {
            let lower = f.to_ascii_lowercase();
            lower.ends_with(".exe") || lower.ends_with(".msu")
        });
        if has_installer {
            crate::store::log_line(&format!(
                "catalog package is an installer only: {update_id}"
            ));
            return Err("this package ships as a vendor installer; UDM does not run downloaded installers automatically".into());
        }
        return Err("the package contains no signed .inf driver".into());
    }
    let meta = serde_json::json!({
        "updateId": update_id,
        "title": title,
        "files": files,
        "infs": infs,
        "signature": signer,
        "sizeBytes": total,
        "downloadedAt": crate::model::now_ms(),
    });
    fs::write(
        meta_path(&dir),
        serde_json::to_vec_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("write meta: {e}"))?;
    crate::store::log_line(&format!(
        "catalog download finished: {update_id} ({} files, {total} bytes, signer={signer})",
        files.len()
    ));
    Ok(CatalogPackage {
        update_id: update_id.to_string(),
        title: title.to_string(),
        dir: dir.to_string_lossy().to_string(),
        files,
        signature: signer,
        infs,
    })
}

fn date_key(value: &str) -> i64 {
    let parts: Vec<i64> = value
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<i64>().ok())
        .collect();
    match parts.as_slice() {
        // the catalog renders dates as M/D/YYYY
        [month, day, year] => year * 10_000 + month * 100 + day,
        _ => 0,
    }
}

fn version_key(value: &str) -> Vec<u64> {
    value
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
}

fn newest(mut updates: Vec<CatalogUpdate>) -> Option<CatalogUpdate> {
    updates.sort_by(|a, b| {
        date_key(&b.date)
            .cmp(&date_key(&a.date))
            .then_with(|| version_key(&b.version).cmp(&version_key(&a.version)))
    });
    updates.into_iter().next()
}

pub fn auto_download(state: &AppState) -> AutoDownloadReport {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let mut report = AutoDownloadReport {
        enabled: settings.auto_download_drivers,
        checked: 0,
        downloaded: Vec::new(),
        skipped: Vec::new(),
    };
    if !settings.auto_download_drivers {
        return report;
    }
    let entries = crate::drivers::scan_internal(None);
    for entry in entries {
        if report.downloaded.len() >= AUTO_MAX_PACKAGES {
            break;
        }
        if !AUTO_PROBLEM_CODES.contains(&entry.problem_code) {
            continue;
        }
        let Some(hardware_id) = entry
            .hardware_ids
            .iter()
            .find(|id| {
                id.starts_with("PCI\\") || id.starts_with("USB\\") || id.starts_with("ACPI\\")
            })
            .or_else(|| entry.hardware_ids.first())
            .cloned()
        else {
            continue;
        };
        report.checked += 1;
        let updates = match search(&hardware_id) {
            Ok(list) => list,
            Err(error) => {
                report.skipped.push(format!("{}: {error}", entry.name));
                continue;
            }
        };
        let Some(update) = newest(updates) else {
            report.skipped.push(format!(
                "{}: no updates in Microsoft Update Catalog",
                entry.name
            ));
            continue;
        };
        if update.size_bytes > AUTO_MAX_PACKAGE {
            report.skipped.push(format!(
                "{}: {} is too large for automatic download",
                entry.name, update.size_text
            ));
            continue;
        }
        match download(&state.data_dir, &update.update_id, &update.title) {
            Ok(package) => report.downloaded.push(AutoDownloadItem {
                device: entry.name,
                title: package.title,
                dir: package.dir,
                infs: package.infs,
                size_bytes: update.size_bytes,
            }),
            Err(error) => report.skipped.push(format!("{}: {error}", entry.name)),
        }
    }
    crate::store::log_line(&format!(
        "catalog auto-download: checked={} downloaded={} skipped={}",
        report.checked,
        report.downloaded.len(),
        report.skipped.len()
    ));
    report
}
