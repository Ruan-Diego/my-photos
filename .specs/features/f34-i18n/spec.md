# F34 — i18n Foundation

Meta do ROADMAP (RNF-020): um sistema de tradução, pt-BR como padrão e estrutura pronta para outros idiomas.

## Requisitos

- **I18N-01**: `t(chave, parâmetros)` devolve o texto do idioma atual: troca `{nome}` pelo valor, formata números do jeito do idioma ("12.345") e escolhe entre singular e plural.
- **I18N-02**: Um idioma novo é um arquivo com as mesmas chaves do pt-BR, registrado em `LOCALES`. O teste confere que cada idioma tem todas as chaves, com os mesmos `{nomes}` e a mesma forma de plural.
- **I18N-03**: Números, datas e ordem alfabética passam pelo módulo de idioma; nenhum arquivo fora de `src/i18n` fixa `"pt-BR"`, e um teste impede que volte.
- **I18N-04**: Já saem do idioma: os nomes dos meses e dos dias da semana, os nomes das seções da barra lateral, as contagens com plural ("1 item"/"N itens", mídias, resultados, miniaturas que faltam), o `lang` da página e a lista de Idioma em Configurações.
- **I18N-05**: Nenhum texto visível muda. Comparamos o texto (visível, `aria-label` e `title`) das 12 telas antes e depois.

- **I18N-06** (segunda parte, 2026-10-28): todos os textos das telas (títulos, botões, `aria-label`, `title`, `placeholder`, avisos, estados vazios, plurais) saem do código para o `pt-BR.ts`, um bloco por tela. Frase com marcação no meio (`<code>`) usa `rich()`, sem partir a frase. Um teste com o parser do TypeScript (`src/i18n/scan.ts`) falha se sobrar texto de tela no código.

## Fora desta feature

As mensagens que vêm do Rust (erros, motivos dos problemas) também ficam para depois. Não há segundo idioma nesta feature: a lista de Idioma em Configurações mostra "Português (Brasil)" e fica travada enquanto houver um só.
