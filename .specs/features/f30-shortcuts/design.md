# F30 — Design

- `src/shortcuts/shortcuts.ts`: funções puras `wheelSize` (passo e limites), `viewerAction` (tecla → ação da foto aberta) e `typing` (foco numa caixa de texto).
- `TimelineScreen`: ouvinte `wheel` não passivo no `.timeline-scroll`; com Ctrl, `preventDefault` (sem zoom da WebView) e chama o mesmo `changeSize` do controle, que mantém o ponto da biblioteca no topo. Um passo a cada 120 ms no máximo, para o touchpad não pular de 1 a 5.
- `PhotoViewer`: o `keydown` passa por `viewerAction`; F chama `requestFullscreen` na raiz `.viewer` (o mesmo que o vídeo faz no quadro dele).
- Sem comando Tauri novo.

## Divergências do canvas

- Nenhuma visual. A única dica nova é o `title` "Também com Ctrl + roda do mouse" no "Tamanho".
