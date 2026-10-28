import assert from "node:assert/strict";
import { test } from "node:test";
import { exportOptions, importText, lastBackupLine, stampDate, type BackupOverview } from "./backup.ts";
import { trashLine } from "./settings.ts";

const overview = (names: string[]): BackupOverview => ({
  folder: "E:\\.myphotos\\backups",
  keep: 7,
  files: names.map((name) => ({ name, bytes: 100 * 1024 * 1024, kind: name.endsWith(".db") ? "database" : "export" })),
});

test("BKUI-01 the time comes from the file name (UTC)", () => {
  assert.equal(stampDate("library-20261028140245.db")?.toISOString(), "2026-10-28T14:02:45.000Z");
  assert.equal(stampDate("dados-20261028100000.json")?.toISOString(), "2026-10-28T10:00:00.000Z");
  assert.equal(stampDate("outro.db"), null);
});

test("BKUI-02 last backup line and the import list", () => {
  assert.equal(lastBackupLine(overview([])), "Nenhum ainda. O app faz um por dia ao abrir.");
  const line = lastBackupLine(overview(["library-20261028140245.db", "library-20261027140245.db", "dados-20261028100000.json"]));
  assert.match(line, /2026/);
  assert.match(line, /2 cópias guardadas \(200 MB\), as 7 mais novas ficam$/);
  const options = exportOptions(overview(["library-20261028140245.db", "dados-20261028100000.json"]));
  assert.deepEqual(options.map((option) => option.name), ["dados-20261028100000.json"]);
  assert.match(options[0].label, /dados-20261028100000\.json$/);
});

test("BKUI-03 import outcome reads as one sentence", () => {
  assert.equal(importText({ favorites: 0, unfavorites: 0, notes: 0, missing: 0 }), "Nada mudou: tudo já estava igual.");
  assert.equal(
    importText({ favorites: 1204, unfavorites: 1, notes: 3, missing: 2 }),
    "Importado: 1.204 favoritos marcados, 1 favorito desmarcado, 3 notas. 2 itens do arquivo não estão nesta biblioteca.",
  );
  assert.equal(trashLine({ databasePath: "E:\\.myphotos\\library.db" }), "E:\\.myphotos\\trash · 30 dias");
  assert.equal(trashLine({ databasePath: null }), "Banco sem arquivo");
});
