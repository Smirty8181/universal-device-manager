export type CapabilityStatus = "working" | "partial" | "unsupported" | "research";

export type Confidence = "verified" | "supported" | "experimental" | "estimated" | "unknown" | "unsupported";

export type BatteryState =
  | "real"
  | "charging"
  | "full"
  | "estimated"
  | "unavailable"
  | "unsupported"
  | "disconnected"
  | "unknown";

export type DeviceKind = "keyboard" | "mouse" | "headset" | "microphone" | "gamepad" | "hid" | "audio" | "other";

export type ConnectionType = "usb" | "wireless" | "bluetooth" | "unknown";

export interface Capability {
  id: string;
  status: CapabilityStatus;
  noteRu: string;
  noteEn: string;
}

export type BatteryStatus = "real" | "charging" | "estimated" | "unavailable" | "disconnected" | "unknown";

export interface BatteryInfo {
  percent: number | null;
  charging: boolean | null;
  source: string;
  status?: BatteryStatus;
  noteRu?: string;
  noteEn?: string;
  updatedAt: number;
}

export interface BatteryRgbRange {
  min: number;
  max: number;
  color: string;
  brightness: number;
  blink: boolean;
}

export interface LightingSnapshot {
  effect: number;
  brightness: number;
  speed: number;
  direction: number;
  color: string;
}

export interface BatteryRgbConfig {
  enabled: boolean;
  ranges: BatteryRgbRange[];
  chargingColor?: string | null;
  chargingBrightness?: number | null;
  layout: string;
  saved?: LightingSnapshot | null;
  lastPercent?: number | null;
  lastColor?: string | null;
}

export interface BatteryRgbView {
  config: BatteryRgbConfig;
  available: boolean;
  percent: number | null;
  charging: boolean | null;
  batteryStatus: string | null;
  reasonRu?: string | null;
  reasonEn?: string | null;
  restorable: boolean;
}

export interface BatteryRgbResult {
  ok: boolean;
  appliedColor?: string | null;
  message: string;
  view: BatteryRgbView;
  status: KeyboardStatus | null;
}

export interface AdapterModuleView {
  id: string;
  label: string;
  status: CapabilityStatus;
  provider: string | null;
  noteRu: string;
  noteEn: string;
}

export interface AdapterBinding {
  id: string;
  adapterId: string;
  adapterLabel: string;
  vid: number;
  pid: number;
  hardwareId?: string | null;
  registryId?: string | null;
  priority: number;
  verification: string;
  source: string;
}

export interface GattServiceInfo {
  uuid: string;
  characteristics: string[];
}

export interface ImportSummary {
  imported: number;
  bindings: number;
  ids: string[];
}

export interface MouseStatus {
  battery: number | null;
  charging: boolean | null;
  dpi: number[];
  activeStage: number | null;
  pollingHz: number | null;
  source: string;
  raw?: string | null;
  noteRu?: string;
  noteEn?: string;
}

export interface MouseCommandResult {
  ok: boolean;
  status: MouseStatus | null;
  message: string;
}

export interface KeyboardStatus {
  pathKind: string;
  profileIndex: number;
  effectId: number;
  hidEffect: number;
  brightness: number;
  speed: number;
  direction: number;
  multiColor: boolean;
  color: string;
  customIndex: number;
  pollingHz: number | null;
  latencyMs: number | null;
  sleepMinutes: number | null;
  debounceMs: number | null;
  batteryRaw: number | null;
  battery: number | null;
  firmware: string | null;
  noteRu?: string;
  noteEn?: string;
}

export interface KeyboardCommandResult {
  ok: boolean;
  status: KeyboardStatus | null;
  message: string;
}

export interface HidInterface {
  path: string;
  usagePage: number;
  usage: number;
  interfaceNumber: number | null;
  product: string | null;
  manufacturer: string | null;
  serial: string | null;
}

export interface DriverInfo {
  version: string | null;
  provider: string | null;
  date: string | null;
  service: string | null;
}

export interface DeviceView {
  id: string;
  kind: DeviceKind;
  name: string;
  vendor: string;
  model: string;
  vid: number | null;
  pid: number | null;
  serial: string | null;
  connection: ConnectionType;
  connectionLabel: string;
  firmware: string | null;
  battery: BatteryInfo | null;
  online: boolean;
  adapterId: string;
  adapterLabel: string;
  registryId?: string | null;
  modules?: AdapterModuleView[];
  containerId?: string | null;
  bluetoothAddress?: string | null;
  capabilities: Capability[];
  interfaces: HidInterface[];
  hardwareIds?: string[];
  driver: DriverInfo | null;
  physicalPath: string | null;
  detectedAt: number;
  updatedAt: number;
}

export interface AudioEndpoint {
  id: string;
  name: string;
  direction: "render" | "capture";
  isDefault: boolean;
  state: string;
}

export interface AudioState {
  endpointId: string;
  volume: number;
  muted: boolean;
  channels: number[];
  channelCount: number;
}

export interface AppSession {
  instanceId: string;
  name: string;
  processId: number;
  volume: number;
  muted: boolean;
  systemSounds: boolean;
}

export interface KeyRemap {
  vk: number;
  target: number | null;
}

export interface ButtonRemap {
  button: number;
  action: "disable" | "button" | "key";
  target: number;
}

export interface InputMap {
  enabled: boolean;
  keys: KeyRemap[];
  buttons: ButtonRemap[];
}

export interface MouseSystem {
  pointerSpeed: number;
  acceleration: boolean;
  wheelLines: number;
  swapButtons: boolean;
}

export interface InterceptEvent {
  at: number;
  kind: "key" | "button";
  from: number;
  to: number | null;
  action: string;
}

export interface InputStatus {
  engineActive: boolean;
  remapping: boolean;
  seenKeys: number;
  swallowedKeys: number;
  seenButtons: number;
  swallowedButtons: number;
  recent: InterceptEvent[];
  map: InputMap;
  system: MouseSystem;
}

export interface PollingStat {
  device: string;
  transport: string;
  kind: "mouse" | "keyboard" | "hid";
  vid: number | null;
  pid: number | null;
  rate: number;
  reports: number;
  windowMs: number;
}

export interface AppSettings {
  language: "ru" | "en";
  accent: "pink" | "violet" | "cyan";
  startMinimized: boolean;
  minimizeToTray: boolean;
  autostart: boolean;
  autoScan: boolean;
  scanIntervalMs: number;
  autoSwitchProfiles: boolean;
  notificationsEnabled: boolean;
  trayBattery: boolean;
  lowBatteryThreshold: number;
  storeHistory: boolean;
  autoUpdateCheck: boolean;
  autoDownloadDrivers: boolean;
  gameMode: boolean;
}

export interface MacroEvent {
  t: number;
  kind: "key" | "mouse";
  label: string;
  down: boolean;
  vk: number;
  x?: number;
  y?: number;
}

export interface Macro {
  id: string;
  name: string;
  createdAt: number;
  events: MacroEvent[];
  repeat: number;
  loop: boolean;
  hotkey: string | null;
}

export interface Profile {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
  apps: string[];
  audio: {
    renderDevicePattern?: string;
    captureDevicePattern?: string;
    volume: number;
    muted: boolean;
    channels?: number[];
    micGain: number;
  };
  rgb: {
    enabled: boolean;
    effect: string;
    color: string;
    secondary: string;
    brightness: number;
    speed: number;
    direction: string;
  };
  mouse: { dpi: number; polling: number; lod: number; debounce: number };
  keyboard: { layout: string; gameMode: boolean; batteryLight: string | null };
}

export interface DriverEntry {
  instanceId: string;
  name: string;
  class: string;
  problemCode: number;
  problemLabel: string | null;
  driverVersion: string | null;
  driverProvider: string | null;
  driverDate: string | null;
  service: string | null;
  driverInf: string | null;
  hardwareIds: string[];
  signature: "windows" | "publisher" | "unsigned" | "unknown";
  usbVid: number | null;
  usbPid: number | null;
}

export interface RegistryLink {
  label: string;
  url: string;
}

export interface DriverCandidate {
  infPath: string;
  publishedName: string | null;
  description: string;
  provider: string;
  version: string;
  date: string | null;
  className: string;
  signature: "windows" | "publisher" | "unsigned" | "unknown";
  exactHwidMatch: boolean;
  newerThanInstalled: boolean;
  isInstalled: boolean;
}

export interface DriverResolution {
  instanceId: string;
  deviceName: string;
  hardwareIds: string[];
  installedVersion: string | null;
  installedInf: string | null;
  candidates: DriverCandidate[];
  sources: RegistryLink[];
}

export interface StoreDriver {
  publishedName: string;
  originalName: string;
  provider: string;
  className: string;
  version: string;
  date: string | null;
}

export interface CatalogUpdate {
  updateId: string;
  title: string;
  products: string;
  classification: string;
  date: string;
  version: string;
  sizeBytes: number;
  sizeText: string;
  url: string;
}

export interface CatalogPackage {
  updateId: string;
  title: string;
  dir: string;
  files: string[];
  signature: "windows" | "publisher" | "unknown";
  infs: string[];
}

export interface AutoDownloadItem {
  device: string;
  title: string;
  dir: string;
  infs: string[];
  sizeBytes: number;
}

export interface AutoDownloadReport {
  enabled: boolean;
  checked: number;
  downloaded: AutoDownloadItem[];
  skipped: string[];
}

export interface FirmwareInfo {
  instanceId: string;
  deviceName: string;
  hardwareIds: string[];
  revision: string | null;
  protocolVersion: string | null;
  driverVersion: string | null;
  driverDate: string | null;
  signature: string;
  knownRevisions: string[];
  sources: RegistryLink[];
  flashable: boolean;
  recoveryDocumented: boolean;
}

export interface RegistrySource {
  label: string;
  url: string;
}

export interface RegistryEntry {
  id: string;
  vendor: string;
  model: string;
  category: DeviceKind;
  adapterId: string;
  adapterLabel: string;
  vidPids: string[];
  hardwareIds: string[];
  hardwareRevision: string | null;
  firmware: string[];
  connections: string[];
  protocolVersion: string | null;
  protocolRu: string;
  protocolEn: string;
  verification: "verified" | "experimental" | "supported" | "unknown" | "unsupported";
  lastChecked: string;
  officialSoftware: string | null;
  capabilities: Capability[];
  sources: RegistrySource[];
}

export interface DbInfo {
  path: string;
  schemaVersion: number;
  migrations: string[];
  deviceCount: number;
  capabilityCount: number;
  sourceCount: number;
  lastUpdated: string | null;
  sizeBytes: number;
}

export interface DbHistoryEntry {
  id: number;
  ts: string;
  action: string;
  deviceId: string | null;
  details: string | null;
}

export interface BatterySnapshot {
  deviceId: string;
  at: number;
  percent: number;
}

export interface TosatMessage {
  id: number;
  kind: "info" | "success" | "warn" | "error";
  title: string;
  body?: string;
}

export interface TelemetryValue {
  key: string;
  labelRu: string;
  labelEn: string;
  value: string | null;
  source: string;
  method: string;
  confidence: Confidence;
  at: number;
}

export interface TelemetryProtocol {
  path: string;
  usagePage: number;
  usage: number;
  interfaceNumber: number | null;
  product: string | null;
  vendorSpecific: boolean;
  inputReports: number[];
  outputReports: number[];
  featureReports: number[];
  fieldCount: number;
  batteryField: string | null;
  descriptorOk: boolean;
  noteRu: string;
  noteEn: string;
}

export interface ReceiverInfo {
  vid: number | null;
  pid: number | null;
  name: string;
  transport: string;
  adapter: string | null;
  protocolKnown: boolean;
  batteryChannel: "real" | "charging" | "unsupported" | "unknown";
  batteryState: BatteryState;
  batteryPercent: number | null;
  interfaces: number;
  vendorInterfaces: number;
  noteRu: string;
  noteEn: string;
}

export interface TelemetryNote {
  level: "info" | "warn" | "error";
  ru: string;
  en: string;
}

export interface TelemetryReport {
  deviceId: string;
  deviceName: string;
  kind: DeviceKind;
  vendor: string;
  model: string;
  connection: ConnectionType;
  connectionLabel: string;
  battery: BatteryInfo;
  batteryState: BatteryState;
  scannedAt: number;
  values: TelemetryValue[];
  protocols: TelemetryProtocol[];
  receiver: ReceiverInfo | null;
  notes: TelemetryNote[];
  source: "live" | "cache";
}

export interface FeatureProbe {
  path: string;
  reportId: number;
  length: number;
  data: string;
  error: string | null;
  at: number;
}

export interface ReportSample {
  at: number;
  reportId: number;
  hex: string;
  repeats: number;
}

export interface CaptureResult {
  path: string;
  durationMs: number;
  samples: ReportSample[];
  error: string | null;
}

export interface LiveKeyEvent {
  at: number;
  vk: number;
  scan: number;
  extended: boolean;
  down: boolean;
  repeat: boolean;
  path: string;
  label: string;
  deviceId: string | null;
}

export interface VisualizerEvent extends LiveKeyEvent {
  keyId: string | null;
  duration: number | null;
  repeats: number;
}

export interface LiveSource {
  path: string;
  label: string;
  deviceId: string | null;
  events: number;
  lastAt: number | null;
  pressed: number[];
}

export interface LiveStatus {
  active: boolean;
  totalEvents: number;
  lastEventAt: number | null;
  sources: LiveSource[];
}

export type AutoClickerButton = "left" | "right" | "middle";
export type AutoClickerMode = "single" | "double" | "hold";

export interface AutoClickerConfig {
  button: AutoClickerButton;
  mode: AutoClickerMode;
  intervalMs: number;
  jitterMs: number;
  limit: number;
  durationMs: number;
  delayMs: number;
  holdMs: number;
}

export interface AutoClickerStatus {
  running: boolean;
  clicks: number;
  elapsedMs: number;
  config: AutoClickerConfig | null;
}

export interface KeyboardSource {
  deviceId: string | null;
  name: string;
  connectionLabel: string;
  online: boolean;
  managed: boolean;
  receiverShared: boolean;
  interfaces: number;
  paths: string[];
}

export interface VisualizerSettings {
  theme: "neon" | "purple" | "cyber" | "minimal" | "matrix" | "custom";
  layout: "auto" | "full" | "tkl" | "75" | "65" | "60";
  iso: boolean;
  accent: string;
  pressedColor: string;
  heldColor: string;
  modifierColor: string;
  glow: number;
  animationMs: number;
  keyLabels: "auto" | "ru" | "en";
  showHistory: boolean;
  historyLimit: number;
  showHeatmap: boolean;
  tracked: "all" | "gaming" | "typing";
  overlayOpacity: number;
  overlaySize: "s" | "m" | "l";
  overlayPosition: "bottom" | "top";
}

export interface SystemInfo {
  appVersion: string;
  osVersion: string;
  admin: boolean;
  dataDir: string;
  logFile: string | null;
  deviceCount: number;
  adapterCount: number;
  startedAt: number;
  tauri: boolean;
}
