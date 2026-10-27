import assert from "node:assert/strict";
import { test } from "node:test";
import {
  countCards,
  headline,
  indexingCard,
  interrupted,
  interruptedCard,
  missingThumbnails,
  thumbnailsCard,
  logLine,
  NO_COUNTS,
  overall,
  percentLine,
  problemsLine,
  pushLines,
  remaining,
  shortPath,
  starting,
  stepStates,
  type Progress,
} from "./progress.ts";

function at(stage: Progress["stage"], done: number, total: number, bytes: [number, number] = [0, 0]): Progress {
  return { ...starting(), stage, done, total, bytesDone: bytes[0], bytesTotal: bytes[1] };
}

test("PROG-01 the title counts the current step", () => {
  assert.equal(headline(starting()), "Procurando arquivos…");
  assert.equal(headline(at("hashing", 1204, 6130)), "1.204 de 6.130 arquivos lidos");
  assert.equal(headline(at("matching", 0, 0)), "Lendo os JSONs do Takeout…");
  assert.equal(headline(at("resolving", 2633, 4247)), "2.633 de 4.247 mídias");
  assert.equal(headline(at("thumbnails", 10, 6130)), "10 de 6.130 miniaturas");
});

test("PROG-02 the six canvas steps follow the stage", () => {
  assert.deepEqual(stepStates("hashing"), ["current", "todo", "todo", "todo", "todo", "todo"]);
  assert.deepEqual(stepStates("matching"), ["done", "current", "current", "todo", "todo", "todo"]);
  assert.deepEqual(stepStates("resolving"), ["done", "done", "done", "current", "current", "todo"]);
  assert.deepEqual(stepStates("thumbnails"), ["done", "done", "done", "done", "done", "current"]);
});

test("PROG-03 the bar moves through the whole run, by bytes while hashing", () => {
  assert.equal(overall(starting()), 0);
  const halfHashed = overall(at("hashing", 1, 100, [50, 100]));
  assert.ok(Math.abs(halfHashed - 0.32) < 1e-9, String(halfHashed));
  // Nothing to hash (a second run): the step counts as done once it is passed.
  assert.ok(overall(at("matching", 0, 0)) > overall(at("hashing", 0, 0)));
  assert.equal(overall(at("thumbnails", 10, 10)), 1);
  const values = [at("scanning", 0, 0), at("hashing", 5, 10, [9, 10]), at("matching", 0, 0), at("resolving", 50, 100), at("thumbnails", 1, 2)].map(overall);
  assert.deepEqual([...values].sort((a, b) => a - b), values);
});

test("PROG-04 time left comes from the pace so far", () => {
  assert.equal(remaining(0.01, 60_000), null);
  assert.equal(remaining(0.5, 5_000), null);
  assert.equal(remaining(0.62, 6 * 60_000), "cerca de 4 min restantes");
  assert.equal(remaining(0.95, 60_000), "menos de 1 min restante");
  assert.equal(remaining(0.1, 20 * 60_000), "cerca de 3 h restantes");
  assert.equal(remaining(0.2, 20 * 60_000), "cerca de 1 h 20 min restantes");
  assert.equal(percentLine(0.62, 6 * 60_000), "62% · cerca de 4 min restantes");
  assert.equal(percentLine(0.005, 1_000), "0%");
});

test("PROG-05 counters, problems and the log read like the canvas", () => {
  const cards = countCards({ ...NO_COUNTS, photos: 2318, videos: 315, dateFromJson: 2587, dateFromExif: 39, jsonInOtherPart: 104, people: 28 });
  assert.deepEqual(
    cards.map((card) => `${card.label}: ${card.value}`),
    ["Fotos: 2.318", "Vídeos: 315", "Data pelo JSON do Takeout: 2.587", "Data pelo EXIF: 39", "JSON em outra parte do export: 104", "Pessoas encontradas: 28"],
  );
  assert.equal(problemsLine(0, false), null);
  assert.equal(problemsLine(12, false), "12 itens não reconhecidos até agora — a indexação continua");
  assert.equal(problemsLine(1, true), "1 item não reconhecido até agora");

  assert.equal(shortPath("Takeout/Google Fotos/Viagem Europa 2022/foto001.jpg"), "Viagem Europa 2022\\foto001.jpg");
  assert.deepEqual(
    logLine({ kind: "paired", relativePath: "a/Viagem Europa 2022/foto001.jpg", jsonName: "foto001.jpg.json", dateSource: null }),
    { text: "pareado · Viagem Europa 2022\\foto001.jpg ← foto001.jpg.json", warn: false },
  );
  assert.deepEqual(logLine({ kind: "noJson", relativePath: "Ruan/camphoto_1271212614.jpg", jsonName: null, dateSource: "exif" }), {
    text: "sem JSON · Ruan\\camphoto_1271212614.jpg → data do EXIF",
    warn: true,
  });
  assert.equal(logLine({ kind: "noJson", relativePath: "v.mp4", jsonName: null, dateSource: "filesystem" }).text, "sem JSON · v.mp4 → data do arquivo");
  const lines = pushLines([], [1, 2, 3, 4, 5].map((n) => ({ text: String(n), warn: false })));
  assert.deepEqual(lines.map((line) => line.text), ["2", "3", "4", "5"]);
});

test("PROG-06 the sidebar card follows the run, a pause and an interrupted first run", () => {
  const card = indexingCard("E:\\", at("resolving", 2633, 4247), false);
  assert.equal(card.title, "Indexando E:\\");
  assert.equal(card.detail, "74% · 2.633 de 4.247 mídias");
  assert.equal(card.action, "Ver progresso");
  assert.equal(indexingCard("E:\\", at("hashing", 3, 10, [0, 10]), false).detail, "2% · 3 de 10 arquivos lidos");
  const pausedCard = indexingCard("E:\\", at("hashing", 5, 10, [5, 10]), true);
  assert.deepEqual(pausedCard, { tone: "warn", title: "Indexação pausada", detail: "32% feito", action: "Continuar" });

  const base = { connected: true, lastIndexedAt: null, mediaCount: 3400 };
  assert.equal(interrupted(base), true);
  assert.equal(interrupted({ ...base, lastIndexedAt: "2026-10-15T19:07:00Z" }), false);
  assert.equal(interrupted({ ...base, mediaCount: 0 }), false);
  assert.equal(interrupted(null), false);
  assert.deepEqual(interruptedCard(3400), { tone: "warn", title: "Indexação incompleta", detail: "3.400 mídias até agora", action: "Continuar" });
});

test("PROG-07 a run closed during the thumbnails asks to continue them", () => {
  const done = { connected: true, lastIndexedAt: "2026-10-26T17:35:36Z", mediaCount: 5210, missingThumbnails: 1234 };
  assert.equal(missingThumbnails(done), 1234);
  assert.equal(missingThumbnails({ ...done, missingThumbnails: 0 }), 0);
  assert.equal(missingThumbnails({ ...done, lastIndexedAt: null }), 0, "first run unfinished: the interrupted card covers it");
  assert.equal(missingThumbnails({ ...done, connected: false }), 0);
  assert.equal(missingThumbnails(null), 0);
  assert.deepEqual(thumbnailsCard(1234), { tone: "warn", title: "Indexação incompleta", detail: "faltam 1.234 miniaturas", action: "Continuar" });
  assert.equal(thumbnailsCard(1).detail, "falta 1 miniatura");
});
