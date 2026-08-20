## What to build

Um subsistema de cálculo contínuo de medição de áudio (VU Meter com RMS, True Peak e detecção de saturação/clipping) e telemetria de rede (RTT em milissegundos, taxa de pacotes e saúde do buffer de jitter), emitido a 60fps via WebSocket para monitoramento visual preciso no console do cliente.

## Acceptance criteria

- [ ] Value object de domínio `VuMeterReading` calculando RMS (energia média quadrática) e Peak absoluto por bloco estéreo (canais L e R).
- [ ] Detecção e sinalização de clipping quando o nível de pico excede 0.995 (-0.04 dBFS) com decaimento suave de pico (peak hold).
- [ ] Mensagens de telemetria periódicas a 60fps transmitidas via WebSocket do servidor para o cliente.
- [ ] Monitoramento de saúde de rede no cliente (RTT/ping de ida e volta, ocupação percentual do buffer de recepção UDP).
- [ ] Testes unitários do cálculo de RMS e Peak com sinais de teste senoidais calibrados.

## Blocked by

- [03 — Motor DSP de Domínio e Sincronização de Faders/Mute via WebSocket](03-dsp-fader-websocket-sync.md)
