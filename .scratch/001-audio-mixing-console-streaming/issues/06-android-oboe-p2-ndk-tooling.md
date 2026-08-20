## What to build

Um adaptador de saída de áudio de ultra-baixa latência para Android utilizando Google `oboe` (AAudio) em modo exclusivo de alta performance, consumindo pacotes do buffer de recepção UDP e direcionando o áudio analógico para o conector 3.5mm P2 (ou adaptador P2/USB-C), junto com scripts e automação de build NDK (`cargo-ndk`) e deploy ADB para dispositivos com Android 12+.

## Acceptance criteria

- [x] Adaptador de playback `OboeAudioPlayback` utilizando a crate `oboe` para saída de áudio em modo exclusivo de baixa latência (`PerformanceMode::LowLatency`).
- [x] Roteamento de áudio configurado para a saída de fone de ouvido / alto-falante externo (conector 3.5mm P2).
- [x] Consumo lock-free de amostras do ring buffer de jitter na callback em tempo real do AAudio.
- [x] Script `scripts/build_android.sh` configurado para compilar a crate `client` para arquitetura `aarch64-linux-android` usando `cargo-ndk` e instalar via `adb`.
- [x] Teste e validação de streaming contínuo sem cortes entre o Mac e o smartphone Android na rede local.

## Blocked by

- [05 — Interface Gráfica Tátil Iced (Console Studio Dark)](05-iced-touch-console-ui.md)
