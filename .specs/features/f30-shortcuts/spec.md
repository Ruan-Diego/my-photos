# F30 — Keyboard Shortcuts

Atalhos de teclado conforme a seção 15.4 do PRD: setas, Esc, F, Ctrl+F, Espaço, Ctrl+scroll.

## O que já existia

| Atalho | Onde | Desde |
| ------ | ---- | ----- |
| Ctrl F | Qualquer tela: abre a busca; na busca, volta o foco à caixa | F22 |
| ← → / Esc | Foto aberta: anterior, próxima, voltar | F19 |
| Espaço / ← → / F / Esc | Vídeo: tocar/pausar, ±5 s, tela cheia, voltar | F20 |

## Requisitos

- **KEY-01** Ctrl + roda do mouse sobre a linha do tempo muda o "Tamanho" um passo por vez (para cima aumenta, para baixo diminui), entre 1 e 5, salvo como o controle. A roda sem Ctrl continua rolando. O zoom da página não acontece.
- **KEY-02** Na foto aberta, F alterna a tela cheia (igual ao vídeo).
- **KEY-03** Esc em tela cheia só sai da tela cheia; fora dela, volta para a grade. Ctrl F não é confundido com F.
- **KEY-04** Teclas digitadas numa caixa de texto não disparam atalhos da foto.

## Fora do escopo

- Atalhos de seleção e de operações de arquivo (chegam com a F48).
- Tela de ajuda listando os atalhos (o canvas não tem); a dica fica no `title` do controle "Tamanho" e no rodapé do vídeo.
