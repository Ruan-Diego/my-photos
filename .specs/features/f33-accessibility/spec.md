# F33 — Accessibility

Meta do ROADMAP (RNF-013): navegação completa por teclado, contraste AA, `aria-label` e leitores de tela. O canvas (`.specs/DESIGN.md` §4) já pede `aria-label` em botões só de ícone, `role="group"` nos grupos e `aria-hidden` no que é decorativo, e as telas F15–F48 seguiram isso.

## Requisitos

- **A11Y-01**: Todo par de texto e fundo do tema atinge WCAG AA nos dois temas: 4,5:1 para texto e 3:1 para o contorno de foco.
- **A11Y-02**: O visualizador e o player de vídeo são modais. Ao abrir, o foco entra neles; o Tab fica preso dentro; ao fechar (Esc), o foco volta para a miniatura que os abriu.
- **A11Y-03**: As miniaturas dizem o que são e quando: "Foto de 10 de agosto de 2022 às 14:37", com a duração do vídeo e "favorita", se for.
- **A11Y-04**: O primeiro Tab mostra "Pular para o conteúdo", que leva o foco para a tela aberta, sem passar pelos itens da barra lateral.
- **A11Y-05**: Os diálogos de Pastas (nova, renomear, mover, lixeira) recebem o foco, fecham com Esc e devolvem o foco ao botão que os abriu. Em Configurações, "Limpar miniaturas" leva o foco para "Cancelar", e "Cancelar" o devolve.
- **A11Y-06**: O axe (WCAG 2.0/2.1 A e AA, mais boas práticas) não acusa nada nas 12 telas, nos dois temas.

## Divergência do canvas

Os pinos do mapa em Locais usam texto branco sobre `--accent-fill`. No escuro isso dá 2,97:1. O novo token `--on-accent` é branco no claro (5,9:1) e `--ground` escuro no escuro (6,1:1), como já faz o ano atual no trilho da linha do tempo.
