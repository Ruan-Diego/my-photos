# F48 — File Operations

Pastas e lixeira do app (artboards `Folders` e `Trash`; AD-008, AD-009, AD-011).

## Requisitos

- **FOP-01** A tela Pastas mostra a árvore como está no disco (tabela `folders`), com o total de mídias de cada pasta, o caminho até a pasta atual e a grade das mídias diretas, com "N itens nesta pasta · M com JSON do Takeout".
- **FOP-02** Nomes de pasta seguem as regras do Windows: sem `\ / : * ? " < > |`, sem ponto no fim, sem nomes reservados (`CON`, `COM1`…) e sem `.myphotos`.
- **FOP-03** "Mover para…" leva cada mídia selecionada com o seu JSON do Takeout. Uma mídia cujo nome já existe no destino fica onde está e entra no aviso. O índice (caminho, JSON, busca) muda na hora.
- **FOP-04** "Nova pasta" cria dentro da pasta atual. "Renomear pasta" renomeia no disco e atualiza todos os caminhos de dentro (mídias, JSONs, subpastas, álbuns do Takeout).
- **FOP-05** "Mover para a lixeira" leva a mídia e o JSON para `E:\.myphotos\trash\<lote>\`. A mídia some de todas as telas (fica `missing_file`), sem perder favorito, pessoas nem miniatura. Restaurar devolve a mídia e o JSON para a pasta de origem e ela volta a aparecer.
- **FOP-06** Uma pasta inteira também vai para a lixeira, com subpastas, e volta inteira.
- **FOP-07** Restaurar nunca sobrescreve. Se já existe um arquivo com o mesmo nome na origem, o item fica na lixeira e o aviso diz por quê. "Restaurar tudo" restaura o que der e lista o resto.
- **FOP-08** Ao abrir o app, o que está na lixeira há 30 dias é apagado de vez: arquivos, miniaturas e linhas do índice.
- **FOP-09** Nada é escrito com uma indexação rodando ou com o disco desconectado.
- **FOP-10** "Mostrar na pasta" das Duplicatas abre a pasta da cópia.

## Divergências do canvas

- Botão extra **"Mover pasta para a lixeira"** (ícone) no cabeçalho de Pastas, com confirmação. O canvas tem uma pasta na Lixeira, mas não mostra de onde se exclui uma pasta.
- O diálogo "Mover para…" lista todas as pastas, com um filtro, e não tem "Nova pasta…". Crie a pasta antes e depois mova.
- A árvore mostra o total da pasta com as subpastas.
- Clique numa miniatura só seleciona; abrir a foto daqui fica para depois.
- A árvore não se fecha: todas as pastas aparecem abertas (no `E:` são 38).
