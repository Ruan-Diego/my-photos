import assert from "node:assert/strict";
import { test } from "node:test";
import { diagnosticLines, durationText, filterProblems, fullPath, reportCards, shortPath, statusFilters, type Problem } from "./problems.ts";

const row = (path: string, status: Problem["status"]): Problem => ({ path, reason: "r", action: "a", status });
const ROWS = [row("a.json", "pending"), row("b.jpg", "pending"), row("c.jpg", "resolved"), row("d.json", "ignored")];

test("PRBUI-01 status chips count and filter like the canvas", () => {
  assert.deepEqual(
    statusFilters(ROWS).map((chip) => chip.label),
    ["Todos · 4", "Pendentes · 2", "Resolvidos · 1", "Ignorados · 1"],
  );
  assert.deepEqual(filterProblems(ROWS, "pending").map((problem) => problem.path), ["a.json", "b.jpg"]);
  assert.equal(filterProblems(ROWS, "all").length, 4);
});

test("PRBUI-02 paths read as Windows paths, shortened", () => {
  assert.equal(shortPath("takeout-1/Takeout/Google Fotos/foto.json"), "…\\Google Fotos\\foto.json");
  assert.equal(shortPath("Ruan/a.jpg"), "Ruan\\a.jpg");
  assert.equal(fullPath("E:\\", "Ruan/a.jpg"), "E:\\Ruan\\a.jpg");
  assert.equal(fullPath(null, "Ruan/a.jpg"), "Ruan\\a.jpg");
});

test("PRBUI-03 report and diagnostics read like the canvas", () => {
  assert.equal(durationText(401), "6 min 41 s");
  assert.equal(durationText(45), "45 s");
  assert.equal(durationText(3720), "1 h 02 min");
  const cards = reportCards({ media: 4247, dateFromJson: 4163, dateFromExif: 71, dateFromFile: 13, jsonInOtherPart: 169, lastDurationSeconds: 401 });
  assert.deepEqual(cards[0], { label: "Mídias indexadas", value: "4.247" });
  assert.deepEqual(cards[5], { label: "Duração da última indexação", value: "6 min 41 s" });
  const diagnostics = { databasePath: "E:\\.myphotos\\library.db", rootPath: "E:\\", thumbnailBytes: 1288490189, thumbnailLimitBytes: 2147483648, ffmpeg: true };
  const lines = diagnosticLines(diagnostics, null);
  assert.equal(lines[0].value, "E:\\.myphotos\\library.db · íntegro");
  assert.equal(lines[1].value, "1,2 GB de 2 GB");
  assert.equal(lines[2].value, "disponível");
  assert.equal(lines[3].value, "E:\\ · conectado");
  assert.equal(diagnosticLines(diagnostics)[0].tone, "unknown");
  assert.equal(diagnosticLines({ ...diagnostics, ffmpeg: false }, "page 3 bad")[0].value, "E:\\.myphotos\\library.db · com defeito: page 3 bad");
});
