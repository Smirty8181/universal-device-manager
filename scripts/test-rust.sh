#!/usr/bin/env bash
# GNU toolchain test binaries import TaskDialogIndirect from comctl32, which
# only resolves with a Common-Controls v6 manifest. Cargo does not embed one
# into test harnesses, so link a small resource object through RUSTFLAGS.
set -euo pipefail
cd "$(dirname "$0")/.."

out="${CARGO_TARGET_DIR:-target}/test-manifest"
mkdir -p "$out"
cat > "$out/udm-test.manifest" <<'XML'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
</assembly>
XML
printf '1 24 "udm-test.manifest"\n' > "$out/udm-test.rc"

if command -v windres >/dev/null 2>&1; then
    ( cd "$out" && windres -O coff -i udm-test.rc -o udm-test.o )
    export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=$(cd "$out" && pwd -W)/udm-test.o"
else
    echo "windres not found, running tests without an embedded manifest" >&2
fi

cargo test --manifest-path src-tauri/Cargo.toml "$@"
