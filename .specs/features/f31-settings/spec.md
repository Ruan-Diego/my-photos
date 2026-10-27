# F31 — Settings UI

Configurações (artboard `Settings`): biblioteca, dados e cache, indexação, aparência e privacidade (RF-059, RF-060, RF-061).

## Requisitos

- **SET-01** A tela mostra a pasta da biblioteca e o nome do volume, o caminho e o tamanho do banco (com o WAL), o uso e o limite do cache de miniaturas, as duas chaves de indexação e as pastas sempre ignoradas.
- **SET-02** "Gerar capa dos vídeos" (ligada por padrão) e "Procurar arquivos novos ao conectar o HD" (desligada por padrão) ficam em `app_settings`, no próprio disco (AD-011). Com a capa desligada, as próximas indexações não chamam o ffmpeg e o cartão "faltam N miniaturas" não conta vídeos; as capas já feitas continuam. Com a busca ligada, o app faz uma busca incremental quando o disco aparece, sem abrir a tela de progresso.
- **SET-03** O limite do cache só aceita 1, 2, 4 (padrão), 5 ou 10 GB. O excesso sai na próxima geração de miniaturas (LRU da F12).
- **SET-04** "Limpar miniaturas" pede confirmação, apaga as linhas de `thumbnails` e a pasta `.myphotos\thumbs` e não roda durante uma indexação. O banco fica intacto. A próxima "Procurar arquivos novos" refaz as miniaturas.
- **SET-05** Aparência: o tema (já existia desde a F15) e as animações ("Seguir o Windows", "Sempre reduzidas", "Completas"), guardadas na janela como o tema. "Sempre reduzidas" desliga transições do CSS e do Motion; "Completas" liga as do Motion mesmo com o Windows pedindo menos movimento.
- **SET-06** Privacidade: aviso fixo de que o app não usa a rede (conferido: nenhum `https://` no front e nenhuma fonte externa).

## Divergências do canvas

- "Procurar arquivos novos ao conectar o HD" vem **desligada**; no canvas aparece ligada. Ligada, cada abertura do app faria uma busca no HD.
- "Reindexar tudo" e "Trocar pasta" ficam desabilitados (decisão da F16; reindexação completa fica para depois, e a busca incremental já relê o que mudou).
- Lixeira: no lugar de `E:\.myphotos\trash · 30 dias` aparece "Chega com as operações de arquivo" (F48).
- Limite do cache: o canvas oferece 1, 2, 5 e 10 GB; o padrão do app é 4 GB (F12), então ele entra na lista como "4 GB (padrão)".
- Idioma: só Português (Brasil), desabilitado até a F34.
- Os itens da navegação lateral são botões que rolam até a seção (o app usa `#/…` para as telas).
