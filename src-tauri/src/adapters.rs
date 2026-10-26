use crate::model::Capability;

fn cap(id: &str, status: &str, ru: &str, en: &str) -> Capability {
    Capability::new(id, status, ru, en)
}

pub fn keyboard_caps() -> Vec<Capability> {
    vec![
        cap("identify", "working", "Определение по VID/PID, HWID и локальной базе.", "Identified by VID/PID, HWID and the local database."),
        cap("connection", "working", "Кабель и 2.4G-ресивер объединяются в одну карточку с обоими транспортами.", "Cable and 2.4G receiver are merged into one card with both transports."),
        cap("lighting", "working", "Подсветка читается и записывается командой 0x02 (feature report 7) с подтверждением чтением. По кабелю; ресивер не принимает feature report 7.", "Lighting is read and written with command 0x02 (feature report 7) and confirmed by read-back. Over cable; the receiver rejects feature report 7."),
        cap("effects", "working", "14 эффектов, яркость и скорость в дискретах 25% применяются к клавиатуре.", "14 effects plus brightness and speed in 25% steps are applied to the keyboard."),
        cap("per_key_rgb", "research", "Раскладка Custom подсвечивается устройством, но запись цветов по клавишам ещё не реализована.", "The device renders the stored Custom layout, but per-key color writing is not implemented yet."),
        cap("battery_level", "partial", "Заряд читается командой 0x8B по кабелю (на тестовой RSQ-20055 получено 93%). На 2.4G-ресивере ответ зависит от прошивки; нулевой первый ответ повторяется автоматически.", "Battery is read with command 0x8B over cable (93% on the tested RSQ-20055). Over the 2.4G receiver the reply depends on firmware; a zero first reply is retried automatically."),
        cap("battery_light_modes", "research", "Заряд через подсветку зависит от протокола подсветки.", "Battery-via-lighting depends on the lighting protocol."),
        cap("key_remap", "research", "Нужен vendor-протокол или системный перехват.", "Needs vendor protocol or system interception."),
        cap("game_mode", "partial", "Встроенный игровой режим не подтверждён; UDM блокирует Win-клавишу системно через собственный хук.", "Built-in game mode is not confirmed; UDM blocks the Windows key system-wide via its own hook."),
        cap("macros", "working", "Системные макросы и горячие клавиши работают без протокола.", "System macros and hotkeys work without the protocol."),
        cap("profiles", "working", "Настройки сохраняются в профили приложения.", "Settings are stored in app profiles."),
    ]
}

pub fn mouse_caps() -> Vec<Capability> {
    vec![
        cap(
            "identify",
            "working",
            "Определение по VID/PID, HWID и локальной базе.",
            "Identified by VID/PID, HWID and the local database.",
        ),
        cap(
            "connection",
            "working",
            "USB-кабель и NearLink-ресивер определяются раздельно.",
            "USB cable and NearLink receiver are identified separately.",
        ),
        cap(
            "battery_level",
            "working",
            "Читается по vendor HID (NearLink), проверено на 0x7768:0x6426.",
            "Read over vendor HID (NearLink); verified on 0x7768:0x6426.",
        ),
        cap(
            "dpi",
            "working",
            "Этапы DPI читаются и записываются с подтверждением чтением.",
            "DPI stages are read and written with read-back confirmation.",
        ),
        cap(
            "polling",
            "working",
            "125/250/500/1000 Гц читаются и записываются с подтверждением.",
            "125/250/500/1000 Hz are read and written with read-back confirmation.",
        ),
        cap(
            "lod",
            "research",
            "LOD требует vendor-протокола.",
            "LOD requires the vendor protocol.",
        ),
        cap(
            "debounce",
            "research",
            "Debounce требует vendor-протокола.",
            "Debounce requires the vendor protocol.",
        ),
        cap(
            "angle",
            "research",
            "Угол сенсора требует vendor-протокола.",
            "Sensor angle requires the vendor protocol.",
        ),
        cap(
            "buttons",
            "research",
            "Переназначение кнопок требует vendor-протокола.",
            "Button remapping requires the vendor protocol.",
        ),
        cap(
            "onboard_profiles",
            "research",
            "Память мыши недоступна без протокола.",
            "Onboard memory is unavailable without the protocol.",
        ),
        cap(
            "macros",
            "working",
            "Системные макросы и горячие клавиши.",
            "System macros and hotkeys.",
        ),
        cap(
            "profiles",
            "working",
            "Настройки сохраняются в профили приложения.",
            "Settings are stored in app profiles.",
        ),
    ]
}

pub fn headset_caps() -> Vec<Capability> {
    vec![
        cap("identify", "working", "Определение как USB-аудио и HID-устройства.", "Detected as a USB audio and HID device."),
        cap("audio_endpoints", "working", "Воспроизведение и микрофон видны в Core Audio.", "Playback and microphone are visible in Core Audio."),
        cap("volume", "working", "Громкость через IAudioEndpointVolume.", "Volume via IAudioEndpointVolume."),
        cap("balance", "working", "Поканальная громкость аудиоэндпоинта.", "Per-channel endpoint volume."),
        cap("mic_gain", "partial", "Усиление зависит от драйвера и доступных узлов эндпоинта.", "Gain depends on the driver and endpoint nodes."),
        cap("eq", "partial", "Через Equalizer APO на уровне Windows; встроенный DSP не документирован.", "Via Equalizer APO at the Windows level; built-in DSP is undocumented."),
        cap("spatial", "partial", "Включается в параметрах Windows, приложение открывает раздел.", "Enabled in Windows settings; the app opens the section."),
        cap("battery_level", "research", "2.4G-ресивер не отдаёт заряд: в HID-каналах 0xFF22/0xFF90 нет батарейного поля. По Bluetooth заряд читается, если наушники публикуют Battery Service.", "The 2.4G receiver does not report battery: its 0xFF22/0xFF90 HID channels have no battery field. Over Bluetooth the charge is read if the headset exposes the Battery Service."),
        cap("monitoring", "unsupported", "Windows не даёт штатного API мониторинга микрофона.", "Windows has no built-in mic monitoring API."),
        cap("default_device", "partial", "Переключение устройства по умолчанию — через параметры Windows.", "Default device switching is done via Windows settings."),
        cap("profiles", "working", "Аудио-настройки сохраняются в профили.", "Audio settings are stored in profiles."),
    ]
}

pub fn audio_caps() -> Vec<Capability> {
    vec![
        cap(
            "identify",
            "working",
            "USB-аудиоустройство определено через PnP.",
            "Detected as a USB audio device via PnP.",
        ),
        cap(
            "audio_endpoints",
            "working",
            "Эндпоинты видны в Core Audio.",
            "Endpoints are visible in Core Audio.",
        ),
        cap(
            "volume",
            "working",
            "Громкость через IAudioEndpointVolume.",
            "Volume via IAudioEndpointVolume.",
        ),
        cap(
            "mic_gain",
            "partial",
            "Зависит от эндпоинта и драйвера.",
            "Depends on the endpoint and driver.",
        ),
        cap(
            "battery_level",
            "research",
            "Источник заряда будет проверен через HID/BLE при опросе.",
            "The battery source is checked over HID/BLE during a probe.",
        ),
        cap(
            "profiles",
            "working",
            "Аудио-настройки сохраняются в профили.",
            "Audio settings are stored in profiles.",
        ),
    ]
}

pub fn generic_caps() -> Vec<Capability> {
    vec![
        cap("identify", "working", "Устройство обнаружено через Windows PnP. Записи в локальной базе нет — функции определены по интерфейсам.", "Discovered through Windows PnP. There is no local database record, so functions are inferred from interfaces."),
        cap("connection", "working", "Тип подключения определяется по PnP-дереву и HID-путям.", "Connection type is derived from the PnP tree and HID paths."),
        cap("diagnostics", "working", "HID-дескрипторы, интерфейсы и пробные чтения доступны в диагностике.", "HID descriptors, interfaces and probe reads are available in diagnostics."),
        cap("battery_level", "research", "Источник заряда не подтверждён; проверяются HID Battery Strength и BLE GATT.", "Battery source is not confirmed; HID Battery Strength and BLE GATT are checked."),
        cap("macros", "working", "Системные макросы и горячие клавиши.", "System macros and hotkeys."),
        cap("profiles", "working", "Настройки сохраняются в профили приложения.", "Settings are stored in app profiles."),
    ]
}

pub fn battery_note(adapter_id: &str, detected: bool) -> (Option<String>, Option<String>) {
    if detected {
        return (
            Some("Найден источник заряда (HID/BLE). Нажмите «Опросить заряд», чтобы прочитать значение.".into()),
            Some("A battery source (HID/BLE) was found. Press \"Probe battery\" to read it.".into()),
        );
    }
    match adapter_id {
        "redsquare" => (
            Some("Заряд читается командой 0x8B; первый ответ после запуска может быть нулевым — выполняется повторное чтение.".into()),
            Some("Battery is read with command 0x8B; the first reply after launch can be zero, so the read is retried.".into()),
        ),
        "ajazz" => (
            Some("Заряд читается по vendor HID NearLink; если мышь спит, ответа не будет.".into()),
            Some("Battery is read over vendor HID NearLink; a sleeping mouse will not answer.".into()),
        ),
        "ioaudio" => (
            Some("2.4G-ресивер не публикует заряд: в HID-каналах 0xFF22/0xFF90 нет батарейного поля. По Bluetooth заряд читается, если наушники публикуют Battery Service.".into()),
            Some("The 2.4G receiver does not publish a charge: the 0xFF22/0xFF90 HID channels have no battery field. Over Bluetooth the charge is read if the headset exposes the Battery Service.".into()),
        ),
        _ => (
            Some("Ни HID Battery Strength, ни BLE GATT не ответили. Заряд помечен как недоступный, а не выдуман.".into()),
            Some("Neither HID Battery Strength nor BLE GATT answered. Battery is marked unavailable, not invented.".into()),
        ),
    }
}
