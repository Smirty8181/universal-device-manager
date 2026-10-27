import type {
  AdapterBinding,
  AppSession,
  AppSettings,
  AudioEndpoint,
  AudioState,
  BatteryInfo,
  BatteryRgbConfig,
  BatteryRgbResult,
  BatteryRgbView,
  BatterySnapshot,
  Capability,
  CatalogPackage,
  CatalogUpdate,
  AutoDownloadReport,
  AutoClickerConfig,
  AutoClickerStatus,
  DbHistoryEntry,
  DbInfo,
  DeviceView,
  DriverCandidate,
  DriverEntry,
  DriverResolution,
  FirmwareInfo,
  GattServiceInfo,
  HidInterface,
  ImportSummary,
  InputMap,
  InputStatus,
  KeyboardCommandResult,
  KeyboardSource,
  KeyboardStatus,
  LiveStatus,
  Macro,
  MouseCommandResult,
  MouseStatus,
  MouseSystem,
  PollingStat,
  Profile,
  RegistryEntry,
  StoreDriver,
  SystemInfo,
  CaptureResult,
  FeatureProbe,
  TelemetryReport,
} from "./types";
import { t } from "./i18n";

const Tauri = (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

export function isTauri(): boolean {
  return Boolean(Tauri);
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke<T>(cmd, args);
}

export async function listen<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const mod = await import("@tauri-apps/api/event");
  return mod.listen<T>(event, (e) => cb(e.payload));
}

export async function emitEvent<T>(event: string, payload?: T): Promise<void> {
  if (!isTauri()) return;
  const mod = await import("@tauri-apps/api/event");
  await mod.emit(event, payload);
}

export async function openExternal(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const mod = await import("@tauri-apps/plugin-opener");
  await mod.openUrl(url);
}

export async function openPath(path: string): Promise<void> {
  if (!isTauri()) return;
  const mod = await import("@tauri-apps/plugin-opener");
  await mod.openPath(path);
}

export async function copyText(text: string): Promise<void> {
  if (isTauri()) {
    await invoke("clipboard_copy", { text });
    return;
  }
  await navigator.clipboard.writeText(text);
}

export function defaultSettings(): AppSettings {
  return {
    language: "ru",
    accent: "pink",
    startMinimized: false,
    minimizeToTray: true,
    autostart: false,
    autoScan: true,
    scanIntervalMs: 3000,
    autoSwitchProfiles: true,
    notificationsEnabled: true,
    trayBattery: true,
    lowBatteryThreshold: 15,
    storeHistory: true,
    autoUpdateCheck: false,
    autoDownloadDrivers: true,
    gameMode: false,
  };
}

function cap(id: string, status: Capability["status"], noteRu: string, noteEn: string): Capability {
  return { id, status, noteRu, noteEn };
}

const keyboardCaps: Capability[] = [
  cap("identify", "working", "Определение по VID/PID, имени и интерфейсам.", "Detected by VID/PID, name and interfaces."),
  cap("connection", "working", "Кабель и 2.4G-ресивер видны как отдельные подключения.", "Cable and 2.4G receiver appear as separate connections."),
  cap("lighting", "research", "Vendor-интерфейс присутствует, протокол подсветки не подтверждён.", "Vendor interface present; lighting protocol not confirmed."),
  cap("per_key_rgb", "research", "Требуется протокол матрицы клавиш.", "Requires the key matrix protocol."),
  cap("effects", "research", "Эффекты сохраняются в профиль, применение ждёт протокола.", "Effects are stored in profiles; applying waits for the protocol."),
  cap("battery_level", "research", "Уровень заряда не приходит стандартным HID.", "Battery level is not reported via standard HID."),
  cap("battery_light_modes", "research", "Заряд через подсветку зависит от протокола подсветки.", "Battery-via-lighting depends on the lighting protocol."),
  cap("key_remap", "research", "Нужен vendor-протокол или системный перехват.", "Needs vendor protocol or system interception."),
  cap("game_mode", "research", "Отключение Win-клавиши на уровне устройства не подтверждено.", "Hardware-level Windows key lock is not confirmed."),
  cap("macros", "working", "Системные макросы и горячие клавиши работают без протокола.", "System macros and hotkeys work without the protocol."),
  cap("profiles", "working", "Настройки сохраняются в профили приложения.", "Settings are stored in app profiles."),
];

const mouseCaps: Capability[] = [
  cap("identify", "working", "Определение по VID/PID и имени устройства.", "Detected by VID/PID and device name."),
  cap("connection", "working", "USB-кабель и NearLink-ресивер определяются раздельно.", "USB cable and NearLink receiver are identified separately."),
  cap("battery_level", "research", "Vendor-протокол заряда не подтверждён.", "Vendor battery protocol not confirmed."),
  cap("dpi", "research", "Настройка DPI требует vendor-протокола.", "DPI control requires the vendor protocol."),
  cap("polling", "research", "Частота опроса не читается стандартными API.", "Polling rate is not readable through standard APIs."),
  cap("lod", "research", "LOD требует vendor-протокола.", "LOD requires the vendor protocol."),
  cap("debounce", "research", "Debounce требует vendor-протокола.", "Debounce requires the vendor protocol."),
  cap("angle", "research", "Угол сенсора требует vendor-протокола.", "Sensor angle requires the vendor protocol."),
  cap("buttons", "research", "Переназначение кнопок требует vendor-протокола.", "Button remapping requires the vendor protocol."),
  cap("onboard_profiles", "research", "Память мыши недоступна без протокола.", "Onboard memory is unavailable without the protocol."),
  cap("macros", "working", "Системные макросы и горячие клавиши.", "System macros and hotkeys."),
  cap("profiles", "working", "Настройки сохраняются в профили приложения.", "Settings are stored in app profiles."),
];

const headsetCaps: Capability[] = [
  cap("identify", "working", "Определение как USB-аудио и HID-устройства.", "Detected as a USB audio and HID device."),
  cap("audio_endpoints", "working", "Воспроизведение и микрофон видны в Core Audio.", "Playback and microphone are visible in Core Audio."),
  cap("volume", "working", "Громкость через IAudioEndpointVolume.", "Volume via IAudioEndpointVolume."),
  cap("balance", "working", "Поканальная громкость аудиоэндпоинта.", "Per-channel endpoint volume."),
  cap("mic_gain", "partial", "Усиление зависит от драйвера и доступных узлов эндпоинта.", "Gain depends on the driver and endpoint nodes."),
  cap("eq", "partial", "Через Equalizer APO на уровне Windows; встроенный DSP не документирован.", "Via Equalizer APO at the Windows level; built-in DSP is undocumented."),
  cap("spatial", "partial", "Включается в параметрах Windows, приложение открывает раздел.", "Enabled in Windows settings; the app opens the section."),
  cap("battery_level", "research", "Аудиоэндпоинт не отдаёт заряд; идёт поиск vendor-команды.", "Audio endpoint does not report battery; vendor command research in progress."),
  cap("monitoring", "unsupported", "Windows не даёт штатного API мониторинга микрофона.", "Windows has no built-in mic monitoring API."),
  cap("default_device", "partial", "Переключение устройства по умолчанию — через параметры Windows.", "Default device switching is done via Windows settings."),
  cap("profiles", "working", "Аудио-настройки сохраняются в профили.", "Audio settings are stored in profiles."),
];

function hidInterface(usagePage: number, usage: number, iface: number | null): HidInterface {
  return {
    path: `\\\\?\\hid#vid_demo&pid_demo&mi_0${iface ?? 0}#{demo}`,
    usagePage,
    usage,
    interfaceNumber: iface,
    product: null,
    manufacturer: null,
    serial: null,
  };
}

export function demoDevices(): DeviceView[] {
  const now = Date.now();
  const noBattery = (noteRu: string, noteEn: string): BatteryInfo => ({
    percent: null,
    charging: null,
    source: "unavailable",
    status: "unavailable",
    noteRu,
    noteEn,
    updatedAt: now,
  });
  const moduleView = (
    id: string,
    status: Capability["status"],
    provider: string | null,
    noteRu: string,
    noteEn: string
  ) => ({ id, label: id, status, provider, noteRu, noteEn });
  return [
    {
      id: "demo-keyboard-342d-e411",
      kind: "keyboard",
      name: "Red Square Alumix TKL",
      vendor: "Red Square",
      model: "RSQ-20055",
      vid: 0x342d,
      pid: 0xe411,
      serial: "28",
      connection: "wireless",
      connectionLabel: "2.4G (RSQ_20055_2.4G)",
      firmware: null,
      battery: noBattery(
        "Клавиатура не отдаёт заряд по стандартному HID. Vendor-протокол исследуется.",
        "The keyboard does not report battery over standard HID. Vendor protocol is being researched."
      ),
      online: true,
      adapterId: "redsquare",
      adapterLabel: "Red Square Adapter",
      modules: [
        moduleView("detection", "working", "windows-pnp + hidapi", "PnP, контейнер, HWID и HID-интерфейсы.", "PnP, container, HWID and HID interfaces."),
        moduleView("battery", "unsupported", "vendor-gsky-0x8B", "0x8B не возвращает процент.", "0x8B returns no percentage."),
        moduleView("rgb", "working", "redsquare-gsky-feature7", "Запись только по кабелю.", "Cable writes only."),
        moduleView("keyboard", "working", "redsquare-gsky", "Подсветка, polling, сон и debounce.", "Lighting, polling, sleep and debounce."),
        moduleView("firmware", "unsupported", null, "Прошивка не выполняется.", "No firmware flashing."),
      ],
      capabilities: keyboardCaps,
      interfaces: [hidInterface(1, 6, 0), hidInterface(1, 128, 1), hidInterface(0xff00, 1, 1)],
      driver: { version: "10.0.22621.4111", provider: "Microsoft", date: null, service: "HidUsb" },
      physicalPath: "USB\\VID_342D&PID_E411\\28",
      detectedAt: now,
      updatedAt: now,
    },
    {
      id: "demo-mouse-7768-6d26",
      kind: "mouse",
      name: "AJAZZ AJ159 NL",
      vendor: "AJAZZ",
      model: "AJ159 NL",
      vid: 0x7768,
      pid: 0x6d26,
      serial: "20240401",
      connection: "wireless",
      connectionLabel: "NearLink (2.4G)",
      firmware: null,
      battery: noBattery(
        "Мышь не отвечает на стандартный запрос заряда. Идёт исследование NearLink-протокола.",
        "The mouse does not answer the standard battery query. NearLink protocol research in progress."
      ),
      online: true,
      adapterId: "ajazz",
      adapterLabel: "AJAZZ Adapter",
      modules: [
        moduleView("detection", "working", "windows-pnp + hidapi", "PnP, контейнер, HWID и HID-интерфейсы.", "PnP, container, HWID and HID interfaces."),
        moduleView("battery", "working", "vendor-nearlink", "Кадр статуса 06 05.", "Status frame 06 05."),
        moduleView("rgb", "research", null, "Протокол подсветки не подтверждён.", "Lighting protocol is not verified."),
        moduleView("mouse", "working", "ajazz-nearlink", "DPI и polling с подтверждением.", "DPI and polling with confirmation."),
        moduleView("firmware", "unsupported", null, "Прошивка не выполняется.", "No firmware flashing."),
      ],
      capabilities: mouseCaps,
      interfaces: [hidInterface(1, 2, 0), hidInterface(1, 128, 1), hidInterface(0xff00, 2, 1)],
      driver: { version: "10.0.22621.6060", provider: "Microsoft", date: null, service: "usbccgp" },
      physicalPath: "USB\\VID_7768&PID_6D26\\20240401",
      detectedAt: now,
      updatedAt: now,
    },
    {
      id: "demo-headset-4c4a-4155",
      kind: "headset",
      name: "IO Graphite 2 Pro Wireless",
      vendor: "IO",
      model: "Graphite 2 Pro",
      vid: 0x4c4a,
      pid: 0x4155,
      serial: "0123456789AB",
      connection: "wireless",
      connectionLabel: "USB-ресивер (Wireless)",
      firmware: null,
      battery: noBattery(
        "Аудиоустройство не отдаёт заряд через стандартный HID.",
        "The audio device does not report battery via standard HID."
      ),
      online: true,
      adapterId: "ioaudio",
      adapterLabel: "IO Audio Adapter",
      modules: [
        moduleView("detection", "working", "windows-pnp + hidapi", "PnP, контейнер, HWID и HID-интерфейсы.", "PnP, container, HWID and HID interfaces."),
        moduleView("battery", "research", null, "Vendor-команда заряда не подтверждена.", "Vendor battery command is not verified."),
        moduleView("rgb", "unsupported", null, "Протокол подсветки не подтверждён.", "Lighting protocol is not verified."),
        moduleView("audio", "working", "windows-coreaudio", "Громкость и баланс через Core Audio.", "Volume and balance through Core Audio."),
        moduleView("firmware", "unsupported", null, "Прошивка не выполняется.", "No firmware flashing."),
      ],
      capabilities: headsetCaps,
      interfaces: [hidInterface(12, 1, 0), hidInterface(0xff00, 1, 0)],
      driver: { version: "10.0.22621.4111", provider: "Microsoft", date: null, service: "HidUsb" },
      physicalPath: "USB\\VID_4C4A&PID_4155\\0123456789AB",
      detectedAt: now,
      updatedAt: now,
    },
  ];
}

const demoAudio: AudioEndpoint[] = [
  { id: "demo-render-headset", name: "Headphones (Graphite 2 Pro Wireless)", direction: "render", isDefault: true, state: "Active" },
  { id: "demo-render-realtek", name: "Speakers (Realtek High Definition Audio)", direction: "render", isDefault: false, state: "Active" },
  { id: "demo-capture-headset", name: "Microphone (Graphite 2 Pro Wireless)", direction: "capture", isDefault: true, state: "Active" },
  { id: "demo-capture-fifine", name: "Microphone (fifine Microphone)", direction: "capture", isDefault: false, state: "Active" },
];

const demoInputStatus: InputStatus = {
  engineActive: false,
  remapping: false,
  seenKeys: 0,
  swallowedKeys: 0,
  seenButtons: 0,
  swallowedButtons: 0,
  recent: [],
  map: { enabled: false, keys: [], buttons: [] },
  system: { pointerSpeed: 10, acceleration: false, wheelLines: 3, swapButtons: false },
};

function demoSettingStore(): AppSettings {
  try {
    const raw = localStorage.getItem("udm.demo.settings");
    if (raw) return { ...defaultSettings(), ...JSON.parse(raw) };
  } catch {}
  return defaultSettings();
}

function demoCollection<T>(key: string): T[] {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T[]) : [];
  } catch {
    return [];
  }
}

function demoPersist(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const api = {
  async listDevices(): Promise<DeviceView[]> {
    if (!isTauri()) return demoDevices();
    return invoke<DeviceView[]>("list_devices");
  },
  async rescanDevices(): Promise<DeviceView[]> {
    if (!isTauri()) {
      await sleep(350);
      return demoDevices();
    }
    return invoke<DeviceView[]>("rescan_devices");
  },
  async probeBattery(deviceId: string): Promise<BatteryInfo | null> {
    if (!isTauri()) {
      await sleep(300);
      return null;
    }
    return invoke<BatteryInfo | null>("probe_battery", { deviceId });
  },
  async mouseStatus(deviceId: string): Promise<MouseStatus> {
    return invoke<MouseStatus>("mouse_status", { deviceId });
  },
  async mouseSetDpi(deviceId: string, stage: number, dpi: number): Promise<MouseCommandResult> {
    return invoke<MouseCommandResult>("mouse_set_dpi", { deviceId, stage, dpi });
  },
  async mouseSetPolling(deviceId: string, hz: number): Promise<MouseCommandResult> {
    return invoke<MouseCommandResult>("mouse_set_polling", { deviceId, hz });
  },

  async batteryRgbGet(deviceId: string): Promise<BatteryRgbView> {
    return invoke<BatteryRgbView>("battery_rgb_get", { deviceId });
  },
  async batteryRgbSave(deviceId: string, config: BatteryRgbConfig): Promise<BatteryRgbView> {
    return invoke<BatteryRgbView>("battery_rgb_save", { deviceId, config });
  },
  async batteryRgbEnable(deviceId: string, enabled: boolean): Promise<BatteryRgbResult> {
    return invoke<BatteryRgbResult>("battery_rgb_enable", { deviceId, enabled });
  },

  async keyboardStatus(deviceId: string): Promise<KeyboardStatus> {
    return invoke<KeyboardStatus>("keyboard_status", { deviceId });
  },
  async keyboardSetLighting(
    deviceId: string,
    effect: number,
    brightness: number,
    speed: number,
    direction: number,
    color: string
  ): Promise<KeyboardCommandResult> {
    return invoke<KeyboardCommandResult>("keyboard_set_lighting", {
      deviceId,
      effect,
      brightness,
      speed,
      direction,
      color,
    });
  },

  async audioList(): Promise<AudioEndpoint[]> {
    if (!isTauri()) return demoAudio;
    return invoke<AudioEndpoint[]>("audio_list");
  },
  async audioState(endpointId: string): Promise<AudioState> {
    if (!isTauri()) {
      return { endpointId, volume: 0.62, muted: false, channels: [0.62, 0.62, 0.62, 0.62, 0.62, 0.62], channelCount: 6 };
    }
    return invoke<AudioState>("audio_state", { endpointId });
  },
  async audioSetVolume(endpointId: string, level: number): Promise<void> {
    if (!isTauri()) return;
    await invoke("audio_set_volume", { endpointId, level });
  },
  async audioSetChannel(endpointId: string, index: number, level: number): Promise<void> {
    if (!isTauri()) return;
    await invoke("audio_set_channel", { endpointId, index, level });
  },
  async audioSetMute(endpointId: string, muted: boolean): Promise<void> {
    if (!isTauri()) return;
    await invoke("audio_set_mute", { endpointId, muted });
  },
  async audioSessions(endpointId: string): Promise<AppSession[]> {
    if (!isTauri()) return [];
    return invoke<AppSession[]>("audio_sessions", { endpointId });
  },
  async audioSessionVolume(endpointId: string, instanceId: string, level: number): Promise<void> {
    if (!isTauri()) return;
    await invoke("audio_session_volume", { endpointId, instanceId, level });
  },
  async audioSessionMute(endpointId: string, instanceId: string, muted: boolean): Promise<void> {
    if (!isTauri()) return;
    await invoke("audio_session_mute", { endpointId, instanceId, muted });
  },

  async inputStatus(): Promise<InputStatus> {
    if (!isTauri()) return demoInputStatus;
    return invoke<InputStatus>("input_status");
  },
  async inputMapSet(map: InputMap): Promise<InputStatus> {
    if (!isTauri()) return { ...demoInputStatus, map };
    return invoke<InputStatus>("input_map_set", { map });
  },
  async inputEnabledSet(enabled: boolean): Promise<InputStatus> {
    if (!isTauri()) return { ...demoInputStatus, map: { ...demoInputStatus.map, enabled } };
    return invoke<InputStatus>("input_enabled_set", { enabled });
  },
  async mouseSystemGet(): Promise<MouseSystem> {
    if (!isTauri()) return demoInputStatus.system;
    return invoke<MouseSystem>("mouse_system_get");
  },
  async mouseSystemSet(value: MouseSystem): Promise<MouseSystem> {
    if (!isTauri()) return value;
    return invoke<MouseSystem>("mouse_system_set", { value });
  },
  async keyCatalog(): Promise<[number, string][]> {
    if (!isTauri()) return [];
    return invoke<[number, string][]>("key_catalog");
  },
  async pollingStats(windowMs: number): Promise<PollingStat[]> {
    if (!isTauri()) return [];
    return invoke<PollingStat[]>("polling_stats", { windowMs });
  },
  async pollingReset(): Promise<void> {
    if (!isTauri()) return;
    await invoke("polling_reset");
  },
  async openSoundSettings(): Promise<void> {
    if (!isTauri()) return;
    await invoke("open_sound_settings");
  },
  async eapoStatus(): Promise<{ installed: boolean; presetDir: string }> {
    if (!isTauri()) return { installed: false, presetDir: "" };
    return invoke("eapo_status");
  },
  async eapoSavePreset(name: string, lines: string[]): Promise<string> {
    if (!isTauri()) return "";
    return invoke<string>("eapo_save_preset", { name, lines });
  },

  async macrosList(): Promise<Macro[]> {
    if (!isTauri()) return demoCollection<Macro>("udm.demo.macros");
    return invoke<Macro[]>("macros_list");
  },
  async macroSave(macro: Macro): Promise<void> {
    if (!isTauri()) {
      const list = demoCollection<Macro>("udm.demo.macros").filter((m) => m.id !== macro.id);
      list.push(macro);
      demoPersist("udm.demo.macros", list);
      return;
    }
    await invoke("macro_save", { macroData: macro });
  },
  async macroDelete(id: string): Promise<void> {
    if (!isTauri()) {
      demoPersist("udm.demo.macros", demoCollection<Macro>("udm.demo.macros").filter((m) => m.id !== id));
      return;
    }
    await invoke("macro_delete", { id });
  },
  async macroRecordStart(): Promise<void> {
    if (!isTauri()) throw new Error(t("common.onlyInApp"));
    await invoke("macro_record_start");
  },
  async macroRecordStop(): Promise<Macro> {
    return invoke<Macro>("macro_record_stop");
  },
  async macroRecordCancel(): Promise<void> {
    await invoke("macro_record_cancel");
  },
  async macroPlay(id: string): Promise<void> {
    await invoke("macro_play", { id });
  },
  async macroStop(): Promise<void> {
    await invoke("macro_stop");
  },
  async hotkeySet(macroId: string, hotkey: string | null): Promise<boolean> {
    if (!isTauri()) return true;
    return invoke<boolean>("macro_set_hotkey", { macroId, hotkey });
  },

  async profilesList(): Promise<Profile[]> {
    if (!isTauri()) return demoCollection<Profile>("udm.demo.profiles");
    return invoke<Profile[]>("profiles_list");
  },
  async profileSave(profile: Profile): Promise<void> {
    if (!isTauri()) {
      const list = demoCollection<Profile>("udm.demo.profiles").filter((p) => p.id !== profile.id);
      list.push(profile);
      demoPersist("udm.demo.profiles", list);
      return;
    }
    await invoke("profile_save", { profile });
  },
  async profileDelete(id: string): Promise<void> {
    if (!isTauri()) {
      demoPersist("udm.demo.profiles", demoCollection<Profile>("udm.demo.profiles").filter((p) => p.id !== id));
      return;
    }
    await invoke("profile_delete", { id });
  },
  async profileActivate(id: string): Promise<void> {
    if (!isTauri()) return;
    await invoke("profile_activate", { id });
  },

  async settingsGet(): Promise<AppSettings> {
    if (!isTauri()) return demoSettingStore();
    return invoke<AppSettings>("settings_get");
  },
  async settingsSet(settings: AppSettings): Promise<void> {
    if (!isTauri()) {
      demoPersist("udm.demo.settings", settings);
      return;
    }
    await invoke("settings_set", { settings });
  },
  async setAutostart(enabled: boolean): Promise<boolean> {
    if (!isTauri()) return true;
    return invoke<boolean>("autostart_set", { enabled });
  },

  async driversScan(): Promise<DriverEntry[]> {
    if (!isTauri()) return [];
    return invoke<DriverEntry[]>("drivers_scan");
  },
  async driverVerify(instanceId: string): Promise<DriverEntry> {
    return invoke<DriverEntry>("driver_verify", { instanceId });
  },
  async driversHistory(): Promise<{ at: number; instanceId: string; name: string; result: string }[]> {
    if (!isTauri()) return [];
    return invoke("drivers_history");
  },
  async driverResolve(instanceId: string): Promise<DriverResolution> {
    return invoke<DriverResolution>("driver_resolve", { instanceId });
  },
  async driverInstall(candidate: DriverCandidate, instanceId: string): Promise<string> {
    return invoke<string>("driver_install", {
      infPath: candidate.infPath,
      instanceId,
      confirm: true,
    });
  },
  async driverInstallElevated(candidate: DriverCandidate, instanceId: string): Promise<string> {
    return invoke<string>("driver_install_elevated", {
      infPath: candidate.infPath,
      instanceId,
      confirm: true,
    });
  },
  async driverStore(): Promise<StoreDriver[]> {
    if (!isTauri()) return [];
    return invoke<StoreDriver[]>("driver_store");
  },
  async driverCatalog(hardwareId: string): Promise<CatalogUpdate[]> {
    return invoke<CatalogUpdate[]>("catalog_search", { hardwareId });
  },
  async driverCatalogDownload(updateId: string, title: string): Promise<CatalogPackage> {
    return invoke<CatalogPackage>("catalog_download", { updateId, title });
  },
  async driverAutoDownload(): Promise<AutoDownloadReport> {
    if (!isTauri()) return { enabled: false, checked: 0, downloaded: [], skipped: [] };
    return invoke<AutoDownloadReport>("driver_auto_download");
  },
  async firmwareInfo(instanceId: string): Promise<FirmwareInfo> {
    return invoke<FirmwareInfo>("firmware_info", { instanceId });
  },

  async systemInfo(): Promise<SystemInfo | null> {
    if (!isTauri()) return null;
    return invoke<SystemInfo>("system_info");
  },
  async registryList(): Promise<RegistryEntry[]> {
    if (!isTauri()) return [];
    return invoke<RegistryEntry[]>("device_registry");
  },
  async dbInfo(): Promise<DbInfo | null> {
    if (!isTauri()) return null;
    return invoke<DbInfo>("db_info");
  },
  async dbHistory(limit = 20): Promise<DbHistoryEntry[]> {
    if (!isTauri()) return [];
    return invoke<DbHistoryEntry[]>("db_history", { limit });
  },
  async registryBindings(): Promise<AdapterBinding[]> {
    return invoke<AdapterBinding[]>("registry_bindings");
  },
  async registryImport(json: string): Promise<ImportSummary> {
    if (!isTauri()) throw new Error("Registry import works in the desktop app only");
    return invoke<ImportSummary>("registry_import", { json });
  },
  async bleInspect(deviceId: string): Promise<GattServiceInfo[]> {
    return invoke<GattServiceInfo[]>("ble_inspect", { deviceId });
  },
  async diagnosticsExport(deviceId: string): Promise<string> {
    return invoke<string>("diagnostics_export", { deviceId });
  },
  async dbBackup(): Promise<string> {
    if (!isTauri()) throw new Error("desktop only");
    return invoke<string>("db_backup");
  },
  async logTail(lines = 120): Promise<string> {
    if (!isTauri()) return "";
    return invoke<string>("log_tail", { lines });
  },
  async hidRead(deviceId: string, path: string, reportId: number, length: number): Promise<{ data: string; error: string | null }> {
    if (!isTauri()) throw new Error("HID lab is available in the desktop app only");
    return invoke("hid_read_feature", { deviceId, path, reportId, length });
  },
  async hidWrite(deviceId: string, path: string, hex: string): Promise<{ data: string; error: string | null }> {
    if (!isTauri()) throw new Error("HID lab is available in the desktop app only");
    return invoke("hid_write_feature", { deviceId, path, hex });
  },
  async hidDescriptor(deviceId: string): Promise<string> {
    if (!isTauri()) throw new Error("HID lab is available in the desktop app only");
    return invoke<string>("hid_descriptor", { deviceId });
  },
  async batteryHistory(): Promise<BatterySnapshot[]> {
    if (!isTauri()) return [];
    return invoke<BatterySnapshot[]>("battery_history");
  },
  async quitApp(): Promise<void> {
    if (!isTauri()) return;
    await invoke("quit_app");
  },
  async trayHide(): Promise<void> {
    if (!isTauri()) return;
    await invoke("tray_hide");
  },
  async trayToggle(): Promise<void> {
    if (!isTauri()) return;
    await invoke("tray_toggle");
  },
  async trayOpen(route?: string): Promise<void> {
    if (!isTauri()) return;
    await invoke("tray_open", { route: route ?? null });
  },

  async telemetryScan(deviceId: string): Promise<TelemetryReport> {
    if (!isTauri()) throw new Error("Telemetry is available in the desktop app only");
    return invoke<TelemetryReport>("telemetry_scan", { deviceId });
  },
  async telemetryCached(deviceId: string): Promise<TelemetryReport | null> {
    if (!isTauri()) return null;
    return invoke<TelemetryReport | null>("telemetry_cached", { deviceId });
  },
  async protocolFeatureSweep(deviceId: string, path: string): Promise<FeatureProbe[]> {
    if (!isTauri()) throw new Error("HID research is available in the desktop app only");
    return invoke<FeatureProbe[]>("protocol_feature_sweep", { deviceId, path });
  },
  async protocolCapture(deviceId: string, path: string, durationMs = 1200): Promise<CaptureResult> {
    if (!isTauri()) throw new Error("HID research is available in the desktop app only");
    return invoke<CaptureResult>("protocol_capture", { deviceId, path, durationMs });
  },
  async liveInputSet(enabled: boolean): Promise<LiveStatus> {
    if (!isTauri()) return { active: enabled, totalEvents: 0, lastEventAt: null, sources: [] };
    return invoke<LiveStatus>("live_input_set", { enabled });
  },
  async liveInputStatus(): Promise<LiveStatus> {
    if (!isTauri()) return { active: false, totalEvents: 0, lastEventAt: null, sources: [] };
    return invoke<LiveStatus>("live_input_status");
  },
  async keyboardSources(): Promise<KeyboardSource[]> {
    if (!isTauri()) return [];
    return invoke<KeyboardSource[]>("keyboard_sources");
  },
  async overlayShow(): Promise<void> {
    if (!isTauri()) return;
    await invoke("overlay_show");
  },
  async overlayHide(stopCapture = false): Promise<void> {
    if (!isTauri()) return;
    await invoke("overlay_hide", { stopCapture });
  },
  async overlayStatus(): Promise<boolean> {
    if (!isTauri()) return false;
    return invoke<boolean>("overlay_status");
  },
  async overlayResize(width: number, height: number, position: "bottom" | "top" = "bottom"): Promise<void> {
    if (!isTauri()) return;
    await invoke("overlay_resize", { width, height, position });
  },
  async autoclickerStart(config: AutoClickerConfig): Promise<AutoClickerStatus> {
    return invoke<AutoClickerStatus>("autoclicker_start", { config });
  },
  async autoclickerStop(): Promise<AutoClickerStatus> {
    if (!isTauri()) return { running: false, clicks: 0, elapsedMs: 0, config: null };
    return invoke<AutoClickerStatus>("autoclicker_stop");
  },
  async autoclickerStatus(): Promise<AutoClickerStatus> {
    if (!isTauri()) return { running: false, clicks: 0, elapsedMs: 0, config: null };
    return invoke<AutoClickerStatus>("autoclicker_status");
  },
};
