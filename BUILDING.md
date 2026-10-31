# Сборка UDM

## Требования

- Windows 10/11 x64;
- Node.js 20+ и npm;
- Rust stable с одним из toolchain:
  - MSVC (рекомендуется): Visual Studio Build Tools с «Desktop development with C++»;
  - GNU: MinGW-w64 (w64devkit или MSYS2);
- WebView2 Runtime и WebView2 SDK (ставится Tauri автоматически);
- интернет для первой загрузки зависимостей и NSIS.

## Frontend

```bash
npm install
npm run build      # vue-tsc + vite build
npm run lint
npm test
```

## Приложение

```bash
npm run tauri build
```

Результат:

- `src-tauri/target/release/udm.exe`;
- `src-tauri/target/release/bundle/nsis/Universal Device Manager_0.1.0_x64-setup.exe`.

Разработка с горячей перезагрузкой:

```bash
npm run tauri dev
```

## Особенности GNU toolchain

- `windres` не понимает путь проекта с не-ASCII символами; `build.rs` копирует иконку в `OUT_DIR` и передаёт её оттуда.
- Перед сборкой закройте запущенный `udm.exe`, иначе компоновщик не перезапишет файл.
- Если целевая папка Cargo находится на другом диске, задайте её явно:
  `export CARGO_TARGET_DIR=C:/Users/<user>/udm-target`.
- Тестовые exe на GNU импортируют `TaskDialogIndirect` из `comctl32.dll` — без манифеста Common-Controls v6 они не запускаются. Используйте:

```bash
bash scripts/test-rust.sh
```

Скрипт собирает маленький ресурс с манифестом и передаёт его через `RUSTFLAGS`. Обычные `cargo build` и `npm run tauri build` этот манифест не подключают.

## Проверки перед релизом

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bash scripts/test-rust.sh
npm run lint
npx vue-tsc -b
npm test
npm run tauri build
```

CI выполняет эти шаги на Windows. Релиз создаётся по тегу `v*`; см. `.github/workflows/release.yml`.

## Portable-сборка

Готовый `udm.exe` требует `WebView2Loader.dll` из `src-tauri/target/release/` (для GNU-сборки файл лежит рядом). Релизный workflow переименовывает его в `Universal Device Manager.exe`, кладёт рядом загрузчик, README, LICENSE и NOTICE и упаковывает всё в `UDM-<tag>-win-x64.zip`.
