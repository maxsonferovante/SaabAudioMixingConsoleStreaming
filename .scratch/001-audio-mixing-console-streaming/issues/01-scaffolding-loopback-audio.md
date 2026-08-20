## What to build

Um workspace Cargo estruturado com Clean Architecture (`protocol`, `core`, `server`, `client`) onde o servidor backend captura o áudio bruto da fonte de entrada, encapsula as amostras de áudio puras (raw PCM float32/int16 estéreo a 48kHz) em datagramas UDP sem alterar os valores do sinal, e transmite em tempo real para o cliente, que consome e reproduz o áudio com integridade de bits através de um buffer circular lock-free (`ringbuf`), validado por teste de integração em loopback.

## Acceptance criteria

- [x] Cargo Workspace raiz configurado com as crates `protocol`, `core`, `server` e `client`.
- [x] Crate `protocol` define o formato binário de ultra-baixa latência do cabeçalho UDP (`AudioPacketHeader`) e pacotes de áudio puro (raw PCM float32/int16 estéreo).
- [x] Crate `core` implementa os ports primários (`ProcessAudioUseCase`) e secundários (`AudioCapturePort`, `AudioStreamerPort`) garantindo pass-through puro de amostras.
- [x] Crate `server` empacota o áudio puro capturado e transmite datagramas UDP via socket de rede local.
- [x] Crate `client` recebe os datagramas UDP e enfileira as amostras puras em buffer circular lock-free para reprodução fiel.
- [x] Teste unitário e de integração em memória validando a transmissão e recepção de buffers de áudio puro com verificação de integridade dos dados (bit-exact) no loopback.

## Blocked by

None — can start immediately
