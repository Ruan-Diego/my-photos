# F35 — App Data Backup

Meta do ROADMAP (RF-066): exportar e importar os dados do app (favoritos, notas) e fazer backup automático do `.db`.

## Requisitos

- **BAK-01**: "Fazer backup agora" grava uma cópia consistente do banco (`VACUUM INTO`) em `.myphotos\backups\library-<hora>.db`. Ficam as 7 mais novas.
- **BAK-02**: Ao abrir, o app faz uma cópia se a mais nova tiver mais de 24 h. Ela roda numa thread com conexão própria, sem atrasar a janela.
- **BAK-03**: "Exportar favoritos e notas" grava `dados-<hora>.json` com o caminho e o SHA-256 de cada favorito e de cada nota. O arquivo também guarda os favoritos do Google Fotos que foram desmarcados no app, porque um índice novo os marcaria de novo.
- **BAK-04**: "Importar" aplica uma exportação da pasta de backups numa transação só. A mídia é achada pelo caminho ou, se mudou de lugar, pelo conteúdo (SHA-256). O resultado diz quantos favoritos foram marcados e desmarcados, quantas notas entraram e quantos itens não estão nesta biblioteca. A importação espera a indexação terminar.
- **BAK-05**: A importação só lê `dados-*.json` de dentro da pasta de backups, por nome, nunca por caminho. O arquivo tem limite de 64 MB e precisa ser do My Photos (`app`, `version`).

## Divergências do canvas

- O canvas não tem o cartão Backup. Ele entra em Configurações, entre "Dados e cache" e "Indexação", no mesmo estilo dos outros cartões, e aparece no menu da esquerda.
- Não há seletor de arquivo, porque o app não tem o plugin de diálogo e não usa rede para baixá-lo. A lista "Importar de" mostra as exportações que estão na pasta de backups. Para trazer de outro computador, basta copiar o arquivo para lá.
- Em "Dados e cache", a linha "Lixeira" agora mostra o caminho real (`E:\.myphotos\trash · 30 dias`) em vez de "Chega com as operações de arquivo".

## Fora do escopo

Restaurar uma cópia do banco pelo app: com o banco aberto, a troca é arriscada. Para restaurar, feche o app e copie `library-<hora>.db` por cima de `library.db`. As notas ainda não têm tela (F47), mas já entram no export e no import.
