const LABELS: Record<string, [string, string]> = {
  identify: ["Определение устройства", "Device identification"],
  connection: ["Тип подключения", "Connection type"],
  lighting: ["Подсветка", "Lighting"],
  per_key_rgb: ["Подсветка отдельных клавиш", "Per-key lighting"],
  effects: ["Эффекты подсветки", "Lighting effects"],
  battery_level: ["Уровень заряда", "Battery level"],
  battery_light_modes: ["Заряд через подсветку", "Battery via lighting"],
  key_remap: ["Переназначение клавиш", "Key remapping"],
  game_mode: ["Игровой режим", "Game mode"],
  macros: ["Макросы", "Macros"],
  profiles: ["Профили", "Profiles"],
  audio_endpoints: ["Аудиоэндпоинты", "Audio endpoints"],
  volume: ["Громкость", "Volume"],
  balance: ["Баланс каналов", "Channel balance"],
  mic_gain: ["Усиление микрофона", "Microphone gain"],
  eq: ["Эквалайзер", "Equalizer"],
  spatial: ["Пространственное звучание", "Spatial audio"],
  monitoring: ["Мониторинг микрофона", "Mic monitoring"],
  default_device: ["Устройство по умолчанию", "Default device"],
  dpi: ["DPI", "DPI"],
  polling: ["Частота опроса", "Polling rate"],
  lod: ["Высота отрыва", "Lift-off distance"],
  debounce: ["Debounce", "Debounce"],
  angle: ["Угол сенсора", "Sensor angle"],
  buttons: ["Кнопки", "Buttons"],
  onboard_profiles: ["Профили в памяти устройства", "Onboard profiles"],
  scroll: ["Колесо прокрутки", "Scroll wheel"],
};

export function capabilityLabel(id: string, locale: "ru" | "en"): string {
  const label = LABELS[id];
  return label ? label[locale === "ru" ? 0 : 1] : id;
}
