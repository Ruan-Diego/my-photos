import assert from "node:assert/strict";
import { test } from "node:test";
import { failedCard, folderDetail, needsOnboarding } from "./flow.ts";

const fresh = {
  connected: true,
  rootPath: null,
  volumeLabel: null,
  mediaCount: 0,
  lastIndexedAt: null,
  missingThumbnails: 0,
  problem: null,
};

test("ONB-02 welcome shows only for a connected disk never indexed", () => {
  assert.equal(needsOnboarding(fresh), true);
  assert.equal(needsOnboarding({ ...fresh, lastIndexedAt: "2026-10-15T14:02:00Z" }), false);
  assert.equal(needsOnboarding({ ...fresh, mediaCount: 3 }), false);
  assert.equal(needsOnboarding({ ...fresh, connected: false }), false);
  assert.equal(needsOnboarding(null), false);
});

test("ONB-01 folder line names the volume and counts the exports", () => {
  assert.equal(
    folderDetail({ rootPath: "E:\\", volumeLabel: "FOTOS", takeoutExports: 3 }),
    "FOTOS · 3 exports do Takeout encontrados",
  );
  assert.equal(folderDetail({ rootPath: "E:\\", volumeLabel: null, takeoutExports: 1 }), "1 export do Takeout encontrado");
  assert.equal(folderDetail({ rootPath: "E:\\", volumeLabel: "", takeoutExports: 0 }), "nenhum export do Takeout encontrado");
});

test("ONB-04 a failed run explains itself and offers to try again", () => {
  const card = failedCard("root_missing");
  assert.equal(card.tone, "warn");
  assert.equal(card.detail, "pasta da biblioteca não encontrada");
  assert.equal(card.action, "Tentar de novo");
  assert.equal(failedCard("algo_novo").detail, "erro: algo_novo");
});
