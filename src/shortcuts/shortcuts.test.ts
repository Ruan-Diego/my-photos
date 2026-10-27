import { test } from "node:test";
import assert from "node:assert/strict";
import { viewerAction, wheelSize, typing } from "./shortcuts.ts";

test("KEY-01 Ctrl + roda muda o tamanho um passo e respeita os limites", () => {
  assert.equal(wheelSize(3, -100), 4);
  assert.equal(wheelSize(3, 3), 2);
  assert.equal(wheelSize(5, -100), 5);
  assert.equal(wheelSize(1, 100), 1);
  assert.equal(wheelSize(2, 0), 2);
});

test("KEY-02 teclas da foto aberta", () => {
  assert.equal(viewerAction("ArrowLeft", false, false), "previous");
  assert.equal(viewerAction("ArrowRight", false, false), "next");
  assert.equal(viewerAction("Escape", false, false), "back");
  assert.equal(viewerAction("f", false, false), "fullscreen");
  assert.equal(viewerAction("F", false, true), "fullscreen");
  assert.equal(viewerAction("x", false, false), null);
});

test("KEY-03 Esc em tela cheia só sai da tela cheia; Ctrl F não é tela cheia", () => {
  assert.equal(viewerAction("Escape", false, true), null);
  assert.equal(viewerAction("f", true, false), null);
});

test("KEY-04 sem elemento focado não conta como digitação", () => {
  assert.equal(typing(null), false);
});
