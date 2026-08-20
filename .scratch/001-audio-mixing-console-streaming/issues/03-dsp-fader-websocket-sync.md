## What to build

Um motor de processamento digital de sinal (DSP) puro em `core` que aplica curva de fader logarítmica de broadcast (-inf a +6dB), rampa suave de ganho anti-pop de 5ms ao mutar/desmutar e botão de Dim (-20dB), integrado a um servidor WebSocket no Mac e cliente WebSocket que sincroniza o estado e os comandos de controle em tempo real com baixa latência.

## Acceptance criteria

- [x] Tipos de valor de domínio imutáveis `DecibelVolume`, `LinearGain` e `MuteState` em `core::domain`.
- [x] Implementação de interpolação linear de ganho em 240 amostras (5ms a 48kHz) para evitar cliques e estalos de transição no áudio.
- [x] Contratos de mensagens de controle WebSocket (`ControlCommandDto`: `SetVolume`, `ToggleMute`, `ToggleDim`) em `protocol`.
- [x] Servidor WebSocket em `server` executado com Tokio processando comandos e atualizando o DSP atômico em tempo de execução.
- [x] Cliente WebSocket em `client` enviando atualizações de volume e recebendo confirmações de estado.
- [x] Testes unitários do domínio verificando atenuação precisa de volume e suavidade do anti-pop sem distorções.

## Blocked by

- [01 — Scaffolding do Workspace e Streaming de Áudio Loopback Puro](01-scaffolding-loopback-audio.md)
