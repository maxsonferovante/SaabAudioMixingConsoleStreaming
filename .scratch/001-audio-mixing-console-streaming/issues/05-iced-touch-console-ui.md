## What to build

Uma interface de usuário de console de mixagem profissional e moderna construída com a biblioteca gráfica `iced`, com tema Dark Studio, apresentando um fader vertical tátil responsivo, medidores estéreo duplos de VU dinâmicos com gradiente colorido (Verde -> Amarelo -> Vermelho), botões iluminados de Mute e Dim, e barra superior com status da conexão e latência.

## Acceptance criteria

- [ ] Componente visual de Fader vertical personalizado com suporte a arraste tátil e teclado, exibindo escala de decibéis (-inf, -60dB a +6dB).
- [ ] Componente visual de VU Meter estéreo duplo com renderização a 60fps, barras em gradiente de cor e indicador de saturação em vermelho.
- [ ] Botões tácteis com iluminação ativa para Mute (vermelho) e Dim (âmbar).
- [ ] Painel superior de telemetria mostrando status de conexão (Conectado / Reconectando), IP do Mac e latência estimada em milissegundos.
- [ ] Suporte a execução em modo Desktop (macOS) e preparação da visualização responsiva para telas verticais de smartphones.

## Blocked by

- [04 — VU Meters Estéreo a 60fps e Telemetria de Rede (RTT, Jitter)](04-vu-meters-telemetry.md)
