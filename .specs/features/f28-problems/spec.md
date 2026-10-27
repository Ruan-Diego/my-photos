# F28 — Error Report & Diagnostics Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Problems`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

A indexação registra problemas (`indexing_errors`, `thumbnail_failures`, datas do arquivo, pareamentos por heurística), mas nada os mostrava: o item "Problemas" estava vazio. O usuário quer saber o que o app não reconheceu e o que fez a respeito, ver os números da última indexação e conferir a saúde do banco (RF-014, RF-062, RF-063, RF-064, RF-065).

## Goals

- [x] Aba "Itens não reconhecidos": caminho, motivo, o que o app fez e status (Pendente, Resolvido, Ignorado), com os filtros "Todos · 37"
- [x] Aba "Relatório da indexação": mídias, data pelo JSON, pelo EXIF, pelo arquivo, JSON em outra parte do export, duração da última indexação
- [x] Aba "Diagnóstico": banco (com "Verificar integridade do banco"), miniaturas em cache, ffmpeg, volume, conexões de rede
- [x] "Exportar relatório": CSV em `E:\.myphotos\relatorios\`

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| F27 Modo offline | AD-011 tirou do MVP: com o disco desconectado não há banco nem miniaturas |
| Marcar um problema como resolvido ou ignorado à mão | O canvas não desenha a ação; o status vem do que o app fez |
| Escolher onde salvar o relatório | Sem plugin de diálogo; fica junto do banco (AD-011) e o caminho aparece na tela |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Quais problemas da varredura | Só os da última indexação terminada (a varredura e o pareamento olham tudo a cada vez) | Os de antes podem já ter sido corrigidos | n |
| Problemas de mídia | Lidos do estado atual: ilegível, EXIF corrompido, sem miniatura, data do arquivo, nome cortado | Valem até o arquivo mudar | n |
| Status | Pendente: o app contornou, mas algo pode faltar; Resolvido: achou outro jeito; Ignorado: nada a mostrar | Igual aos exemplos do canvas | n |
| Linhas na tabela | Até 500; o CSV tem todas | 406 vídeos com data do arquivo já passam disso | n |
| ffmpeg | "disponível" / "não encontrado · vídeos sem capa nem duração" | Ainda não é empacotado (decisão aberta) | n |
| Conexões de rede | "nenhuma", fixo | O app não abre conexões (RNF-005) | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| PRB-01 | Problemas da última indexação terminada e do estado das mídias, pendentes primeiro |
| PRB-02 | Relatório: contagens por fonte de data, JSON em outra parte, duração da última indexação completa; integridade do banco |
| PRB-03 | CSV com BOM, campos entre aspas quando preciso, status em português |
| PRBUI-01 | Filtros por status com contagem |
| PRBUI-02 | Caminhos no formato do Windows, encurtados |
| PRBUI-03 | Relatório e diagnóstico com os textos do canvas |
