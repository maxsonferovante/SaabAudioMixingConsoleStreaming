## What to build

Um adaptador de captura de áudio nativo para macOS no backend (`server`) que se integra com a API do ScreenCaptureKit para capturar todo o áudio digital emitido pelo sistema operacional e por aplicativos ativos (jogos, música, navegadores, chamadas) sem a necessidade de drivers de terceiros (como BlackHole), alimentando o pipeline de streaming em tempo real.

## Acceptance criteria

- [x] Implementação do adaptador `ScreenCaptureKitAudioCapture` que implementa a trait `AudioCapturePort`.
- [x] Inicialização da captura de áudio digital global do sistema no macOS 12.3+ / 13+.
- [x] Conversão e alinhamento de taxa de amostragem para 48kHz estéreo float32 PCM.
- [x] Fallback transparente para dispositivos `cpal` locais em caso de execução fora do macOS ou permissões não concedidas.
- [x] Teste de inicialização e captura de amostras reais do sistema validado no backend.

## Blocked by

- [01 — Scaffolding do Workspace e Streaming de Áudio Loopback Puro](01-scaffolding-loopback-audio.md)
