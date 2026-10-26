# F21 — Details Panel Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Viewer`**.

## Rust

- `commands/details.rs`: comando `media_panel_details(id)` → `MediaPanel` (fonte da data, latitude/longitude de `media_locations`, pessoas de `media_people` em ordem alfabética, pasta como caminho do Windows, duração, nome do JSON pareado e confiança, câmera de `Make`/`Model` do EXIF guardado, SHA-256). Permissão `allow-media-panel-details` (teste `acl01`).

## React

- `src/viewer/panel.ts`: textos sem React (`fullDate`, `dateSource`, `coordinates`, `fileRows`, `technicalRows`).
- `src/viewer/DetailsPanel.tsx`: o `<aside>` de 340 px; `useDetailsOpen` guarda aberto/fechado no `localStorage`.
- `Viewer.tsx` e `VideoPlayer.tsx`: o conteúdo vai para `.viewer-main` e o painel entra ao lado; o botão ⓘ deixa de ser inativo (`aria-pressed`, fundo `--viewer-button` quando aberto).
- Tokens novos `--viewer-panel`, `--viewer-grid`, `--viewer-ok-*`, `--viewer-warn-*`, iguais nos dois temas.

## Fora do canvas

- Textos de vazio: "Sem localização no arquivo nem no JSON", "Ninguém marcado no Google Fotos", "Data não encontrada no arquivo nem no JSON".
- Selo da data do arquivo em tom de alerta, sem o ✓.
- Linha "Duração" no vídeo.
