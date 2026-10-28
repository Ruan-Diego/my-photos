import { test } from "node:test";
import assert from "node:assert/strict";
import { cacheOptions, privacyNote, cacheUsage, databaseLine, parseMotion, reducedMotion, videoCoversNote, volumeLine } from "./settings.ts";

const GB = 1024 ** 3;

test("SETUI-01 linhas da biblioteca e do banco", () => {
  assert.equal(volumeLine({ volumeLabel: "FOTOS" }), "FOTOS · reconhecido pelo número de série, mesmo se a letra mudar");
  assert.equal(volumeLine({ volumeLabel: null }), "Disco sem nome · reconhecido pelo número de série, mesmo se a letra mudar");
  assert.equal(databaseLine({ databasePath: "E:\\.myphotos\\library.db", databaseBytes: 184 * 1024 * 1024 }), "E:\\.myphotos\\library.db · 184 MB");
  assert.equal(databaseLine({ databasePath: "E:\\.myphotos\\library.db", databaseBytes: null }), "E:\\.myphotos\\library.db");
});

test("SETUI-02 uso do cache e opções do limite", () => {
  const usage = cacheUsage({ thumbnailBytes: 1.2 * GB, thumbnailLimitBytes: 2 * GB });
  assert.equal(usage.text, "1,2 GB usados de 2 GB");
  assert.equal(usage.fraction, 0.6);
  assert.equal(cacheUsage({ thumbnailBytes: 3 * GB, thumbnailLimitBytes: 2 * GB }).fraction, 1);
  const options = cacheOptions({ cacheLimits: [GB, 2 * GB, 4 * GB], thumbnailLimitBytes: 4 * GB });
  assert.deepEqual(options.map((o) => o.label), ["1 GB", "2 GB", "4 GB (padrão)"]);
  const odd = cacheOptions({ cacheLimits: [GB, 4 * GB], thumbnailLimitBytes: 3 * GB });
  assert.deepEqual(odd.map((o) => o.bytes), [GB, 3 * GB, 4 * GB]);
});

test("SETUI-03 aviso das capas de vídeo", () => {
  assert.equal(videoCoversNote({ videoCovers: true, ffmpeg: true }), null);
  assert.match(videoCoversNote({ videoCovers: true, ffmpeg: false }) ?? "", /ffmpeg/);
  assert.match(videoCoversNote({ videoCovers: false, ffmpeg: true }) ?? "", /continuam/);
});

test("SETUI-04 animações", () => {
  assert.equal(parseMotion("reduced"), "reduced");
  assert.equal(parseMotion("x"), "system");
  assert.equal(reducedMotion("system"), "user");
  assert.equal(reducedMotion("reduced"), "always");
  assert.equal(reducedMotion("full"), "never");
});

test("UPD-01 Privacidade diz quando a busca de atualizações usa a rede", () => {
  assert.match(privacyNote({ updateChecks: false }), /não faz nenhuma conexão de rede/);
  assert.match(privacyNote(null), /não faz nenhuma conexão de rede/);
  assert.match(privacyNote({ updateChecks: true }), /única conexão de rede é a busca de atualizações/);
});
