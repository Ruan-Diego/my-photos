# F36 — Security Hardening

Meta do ROADMAP (RNF-005, RNF-007, RNF-008): validar entradas, isolar o parsing de arquivos, `PRAGMA integrity_check`, usar só as permissões necessárias e não usar rede por padrão.

## Requisitos

- **SEC-01 (parsing isolado)**: os leitores de arquivo (`image` nas miniaturas e no hash perceptual, `kamadak-exif` e `imagesize` nos metadados) rodam dentro de `isolate::contained`. Se um arquivo malformado fizer um deles entrar em pânico, só aquele arquivo falha. A miniatura vira "corrompido", o EXIF vira "corrompido" e o hash fica para a próxima vez. O app e a indexação continuam. Para isso, o perfil release passa de `panic = "abort"` para `panic = "unwind"`.
- **SEC-02 (integridade antes da cópia)**: antes de cada cópia do banco, a automática e a do botão, o app roda `PRAGMA integrity_check`. Se o banco estiver danificado, a cópia não é feita e o erro diz isso. Assim uma cópia ruim nunca empurra uma boa para fora das 7 guardadas. A verificação manual em Problemas (F28) continua igual.
- **SEC-03 (permissões mínimas)**: a janela recebe só os comandos do app e `core:event:default`, usado pelo progresso da indexação. Saem `core:default` (janelas, menus, bandeja, caminhos) e o plugin opener, que nenhuma tela usava. O plugin sai do Rust e do npm.
- **SEC-04 (zero rede)**: a CSP, que antes era `null`, passa a ser restrita:
  - scripts só do próprio app;
  - imagens e vídeos só dos protocolos `thumb` e `media`;
  - `connect-src` só o IPC;
  - sem `eval`, sem objetos, sem frames e sem formulários enviados.
  O `Cargo.toml` não tem crate de rede (o `reqwest` do Tauri só entra no Android e no iOS). Um teste falha se isso mudar. O auto-update da F37 terá de entrar como exceção explícita e opcional.
- **SEC-05 (validação de entrada)**: auditoria dos 37 comandos (tabela em `design.md`). Os que recebem texto ou números já validavam: nomes de pasta, pastas conhecidas no índice, chaves e limites de configuração por lista fechada, importação só por nome dentro da pasta de backups, `Range` dos vídeos, ids numéricos nos protocolos e busca com parâmetros SQL. Não faltou nenhuma validação.

## Divergências do canvas

Nenhuma: a F36 não muda nenhuma tela.

## Fora do escopo

Isolar os decodificadores em outro processo. O `catch_unwind` cobre os pânicos, mas não um decodificador que trave ou estoure a memória. O `image` já limita cada alocação a 512 MB por padrão.
