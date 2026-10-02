import assert from "node:assert/strict";
import { test } from "node:test";
import { healthView } from "./view.ts";

test("success screen shows the schema version", () => {
  const screen = healthView({ appName: "My Photos", schemaVersion: 1 }, null);

  assert.equal(screen.ready, "My Photos está pronto.");
  assert.equal(screen.schema, "Banco: esquema v1");
  assert.equal(screen.libraries, "three, gsap, animejs, motion");
  assert.equal(screen.failure, null);
});

test("invoke error renders Falha with the message and hides the success strings", () => {
  const screen = healthView(null, "disco E: não encontrado");

  assert.equal(screen.failure, "Falha: disco E: não encontrado");
  assert.equal(screen.ready, null);
  assert.equal(screen.schema, null);
  assert.equal(screen.libraries, null);
});

test("schemaVersion that is not an integer of at least 1 is invalid", () => {
  for (const schemaVersion of [0, 1.5]) {
    const screen = healthView({ appName: "My Photos", schemaVersion }, null);

    assert.equal(screen.failure, "Falha: schemaVersion inválido");
    assert.equal(screen.ready, null);
    assert.equal(screen.schema, null);
    assert.equal(screen.libraries, null);
  }
});
