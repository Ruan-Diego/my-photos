import assert from "node:assert/strict";
import { test } from "node:test";
import type { Item } from "../timeline/timeline.ts";
import { coordinates, dateSource, fileRows, fullDate, technicalRows, type MediaPanel } from "./panel.ts";

// 2022-08-10T13:37Z em Lisboa (+01:00).
const ITEM: Item = { id: 1, ratio: 1.33, video: false, favorite: false, duration: null, time: 1660138620, offset: 60 };

const PANEL: MediaPanel = {
  id: 1,
  dateSource: "takeout_json",
  latitude: 38.7139,
  longitude: -9.1394,
  people: ["Ana"],
  folder: "E:\\Takeout\\Google Fotos\\Viagem Europa 2022",
  durationMs: null,
  jsonFilename: "IMG_20220810_143700.jpg.supplemental-metadata.json",
  jsonMatch: "exact",
  camera: null,
  sha256: null,
};

test("DET-01 date, source and place read like the canvas", () => {
  assert.equal(fullDate(ITEM), "10 de agosto de 2022, 14:37");
  assert.equal(fullDate({ ...ITEM, time: null }), null);
  assert.deepEqual(dateSource("takeout_json"), { label: "Do JSON do Takeout", tone: "ok" });
  assert.equal(dateSource("filesystem")?.tone, "warn");
  assert.equal(dateSource(null), null);
  assert.equal(coordinates(38.7139, -9.1394), "38,7139° N · 9,1394° O");
  assert.equal(coordinates(-23.5505, 46.6333), "23,5505° S · 46,6333° L");
  assert.equal(coordinates(null, 1), null);
});

test("DET-02 file and technical rows say what is known", () => {
  const info = { filename: "IMG_20220810_143700.jpg", width: 4032, height: 3024, sizeBytes: 3565158 };
  assert.deepEqual(fileRows(info, PANEL), [
    ["Nome", "IMG_20220810_143700.jpg"],
    ["Pasta", "E:\\Takeout\\Google Fotos\\Viagem Europa 2022"],
    ["Dimensões", "4032 × 3024"],
    ["Tamanho", "3,4 MB"],
  ]);
  const video = fileRows({ ...info, width: null, sizeBytes: null }, { ...PANEL, durationMs: 138000 });
  assert.deepEqual(video.map(([label]) => label), ["Nome", "Pasta", "Duração"]);
  assert.deepEqual(video[2], ["Duração", "2:18"]);
  assert.deepEqual(technicalRows(PANEL), [
    ["JSON pareado", "IMG_20220810_143700.jpg.supplemental-metadata.json"],
    ["Pareamento", "exato"],
    ["Câmera", "não informada"],
    ["SHA-256", "ainda não calculado"],
  ]);
});
