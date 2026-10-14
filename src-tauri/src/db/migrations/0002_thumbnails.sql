-- F12: one row per thumbnail on disk, under `.myphotos/thumbs/` (AD-011).
-- The path is derived from (media_id, size); `source_fingerprint` says which version of
-- the original it was made from, so a changed file gets a new one.
CREATE TABLE thumbnails (
  media_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
  size TEXT NOT NULL CHECK (size IN ('small','medium','large')),
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  bytes INTEGER NOT NULL,
  source_fingerprint TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_accessed_at TEXT NOT NULL,
  PRIMARY KEY (media_id, size)
);
CREATE INDEX idx_thumbnails_lru ON thumbnails(last_accessed_at);

-- F12: originals no thumbnail can be made from (HEIC, RAW, corrupt). Not retried until
-- the file changes (fingerprint).
CREATE TABLE thumbnail_failures (
  media_id INTEGER PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
  source_fingerprint TEXT NOT NULL,
  reason TEXT NOT NULL CHECK (reason IN ('unsupported_format','corrupt_media','unreadable')),
  created_at TEXT NOT NULL
);
