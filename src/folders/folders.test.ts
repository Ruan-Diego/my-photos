import { test } from "node:test";
import assert from "node:assert/strict";
import {
  breadcrumb,
  folderFromHash,
  folderHash,
  folderSummary,
  outcomeText,
  purgeText,
  select,
  targetLabel,
  trashName,
  trashOrigin,
  type TrashItem,
} from "./folders.ts";

test("FOLUI-01 endereço da pasta vai e volta", () => {
  assert.equal(folderHash(""), "#/pastas");
  const hash = folderHash("Takeout/Viagem Europa 2022");
  assert.equal(hash, "#/pastas/Takeout%2FViagem%20Europa%202022");
  assert.equal(folderFromHash(hash), "Takeout/Viagem Europa 2022");
  assert.equal(folderFromHash("#/pastas"), "");
  assert.equal(folderFromHash("#/pastas/%E0%A4%A"), "");
});

test("FOLUI-02 caminho e rótulos", () => {
  assert.deepEqual(breadcrumb("a/b", "E:\\"), [
    { label: "E:\\", path: "" },
    { label: "a", path: "a" },
    { label: "b", path: "a/b" },
  ]);
  assert.equal(targetLabel("Organizadas/Portugal", "E:\\"), "Organizadas \\ Portugal");
  assert.equal(targetLabel("", "E:\\"), "E:\\");
});

test("FOLUI-03 seleção com clique e Shift", () => {
  const order = [1, 2, 3, 4, 5];
  let selected = select(new Set(), order, 2, null, false);
  assert.deepEqual([...selected], [2]);
  selected = select(selected, order, 4, 2, true);
  assert.deepEqual([...selected].sort(), [2, 3, 4]);
  selected = select(selected, order, 3, 4, false);
  assert.deepEqual([...selected].sort(), [2, 4]);
});

test("FOLUI-04 textos de resultado", () => {
  assert.equal(folderSummary([{ id: 1, filename: "a", video: false, hasJson: true }, { id: 2, filename: "b", video: false, hasJson: false }]), "2 itens nesta pasta · 1 com JSON do Takeout");
  assert.equal(outcomeText({ done: 3, skipped: [] }, "movido"), "3 itens movidos.");
  assert.equal(outcomeText({ done: 1, skipped: [] }, "movido", " para a lixeira"), "1 item movido para a lixeira.");
  assert.equal(
    outcomeText({ done: 1, skipped: [["b.jpg", "já existe b.jpg no destino"], ["c.jpg", "x"]] }, "movido"),
    "1 item movido. Ficou como estava: b.jpg (já existe b.jpg no destino) e mais 1.",
  );
});

test("FOLUI-05 lixeira: dias e nomes", () => {
  const now = Date.parse("2026-10-27T20:00:00Z");
  assert.deepEqual(purgeText("2026-11-25T20:00:00Z", now), { text: "apagado em 29 dias", warn: false });
  assert.deepEqual(purgeText("2026-10-28T19:00:00Z", now), { text: "apagado amanhã", warn: true });
  assert.deepEqual(purgeText("2026-10-27T10:00:00Z", now), { text: "apagado hoje", warn: true });
  const folder: TrashItem = { id: 1, kind: "folder", mediaId: null, name: "Rascunhos", folder: "Organizadas", items: 42, trashedAt: "", purgeAfter: "" };
  assert.equal(trashName(folder), "Pasta “Rascunhos” · 42 itens");
  assert.equal(trashOrigin(folder, "E:\\"), "Estava em Organizadas");
  assert.equal(trashOrigin({ ...folder, folder: "Google Fotos/Fotos de 2021" }, "E:\\"), "Estava em Google Fotos\\Fotos de 2021");
  assert.equal(trashOrigin({ ...folder, folder: "" }, "E:\\"), "Estava em E:\\");
});
