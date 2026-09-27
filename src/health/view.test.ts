import assert from "node:assert/strict";
import { test } from "node:test";
import { healthView } from "./view.ts";

test("success screen uses the spec strings", () => {
  const screen = healthView({ appName: "My Photos", sqliteOk: true }, null);

  assert.equal(screen.ready, "My Photos está pronto.");
  assert.equal(screen.sqlite, "SQLite: ok");
  assert.equal(screen.libraries, "three, gsap, animejs, motion");
  assert.equal(screen.failure, null);
});

test("invoke error renders Falha and hides the success strings", () => {
  const screen = healthView(null, "boom");

  assert.equal(screen.failure, "Falha: boom");
  assert.equal(screen.ready, null);
  assert.equal(screen.sqlite, null);
  assert.equal(screen.libraries, null);
});

test("sqliteOk false renders the invalid payload text", () => {
  const screen = healthView({ appName: "My Photos", sqliteOk: false }, null);

  assert.equal(screen.failure, "Falha: sqliteOk inválido");
  assert.equal(screen.ready, null);
  assert.equal(screen.sqlite, null);
  assert.equal(screen.libraries, null);
});
