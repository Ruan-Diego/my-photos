# F36 — Validation

| Requisito | Teste |
| --------- | ----- |
| SEC-01 | `isolate::tests::sec01_*`: um parser que entra em pânico devolve `None` e a thread segue para o próximo arquivo |
| SEC-02 | `backup::tests::sec02_*`: com lixo no cabeçalho de uma página do banco, a cópia é recusada ("Tree 40 page 84: btreeInitPage() returns error code 11") e a cópia boa continua a mais nova. Sem a verificação, o teste falha |
| SEC-03 | `commands::tests::sec03_*`: a capability só tem `allow-*` e `core:event:default`; `acl01` continua passando |
| SEC-04 | `commands::tests::sec04_*`: a CSP não tem origem remota, `*`, `https:` nem `unsafe-eval`; o `Cargo.toml` não tem `reqwest`, `hyper`, `ureq`, `curl`, nem os plugins http, updater ou opener. Playwright: o build servido com a CSP (e o hash do script inline, como o Tauri faz) abre as 13 telas, entre elas o visualizador de foto e de vídeo, sem nenhuma violação. As fontes Hanken Grotesk carregam e as miniaturas vêm de `http://thumb.localhost`. Controle negativo: sem o hash e sem `thumb:`, as violações aparecem. Captura `csp-timeline.png` em `/mnt/project-files/f36-security/` |
| SEC-05 | Tabela em `design.md`; os testes de validação já existentes (`fileops`, `bak04`, `video01`, settings) continuam passando |

Pendente no Windows:
- abrir o app instalado e ver miniaturas, vídeos e fontes com a CSP;
- confirmar que o progresso da indexação ainda chega só com `core:event:default`.
