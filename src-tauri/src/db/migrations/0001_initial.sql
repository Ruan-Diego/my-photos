CREATE TABLE libraries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL,
  volume_id TEXT,
  volume_label TEXT,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','offline','archived')),
  created_at TEXT NOT NULL,
  last_indexed_at TEXT
);

-- F04: pastas conhecidas, inclusive vazias (AD-008). A raiz não tem linha.
CREATE TABLE folders (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  relative_path TEXT NOT NULL COLLATE NOCASE,
  created_at TEXT NOT NULL,
  UNIQUE (library_id, relative_path)
);

CREATE TABLE media_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  relative_path TEXT NOT NULL COLLATE NOCASE,   -- F04: NTFS não diferencia caixa
  filename TEXT NOT NULL,
  extension TEXT NOT NULL,
  media_type TEXT NOT NULL CHECK (media_type IN ('photo','video')),
  size_bytes INTEGER,
  file_hash_sha256 TEXT,
  perceptual_hash TEXT,
  width INTEGER,
  height INTEGER,
  orientation INTEGER,
  duration_ms INTEGER,
  taken_at TEXT,
  taken_at_source TEXT CHECK (taken_at_source IN ('takeout_json','exif','filesystem')),
  taken_at_timezone TEXT,
  fs_created_at TEXT,
  fs_modified_at TEXT,
  has_takeout_json INTEGER NOT NULL DEFAULT 0 CHECK (has_takeout_json IN (0,1)),
  json_relative_path TEXT,
  json_match_confidence TEXT CHECK (json_match_confidence IN ('exact','heuristic','none')),
  title TEXT,
  description TEXT,
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0,1)),
  is_hidden INTEGER NOT NULL DEFAULT 0 CHECK (is_hidden IN (0,1)),
  indexing_status TEXT NOT NULL DEFAULT 'ok'
    CHECK (indexing_status IN ('ok','error','orphan_json','missing_file')),
  indexing_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (library_id, relative_path)
);
CREATE INDEX idx_media_taken_at ON media_items(library_id, taken_at);
CREATE INDEX idx_media_hash ON media_items(file_hash_sha256);
CREATE INDEX idx_media_type ON media_items(library_id, media_type);

CREATE TABLE media_locations (
  media_id INTEGER PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
  latitude REAL,
  longitude REAL,
  altitude REAL,
  source TEXT CHECK (source IN ('takeout_geoData','takeout_geoDataExif','exif')),
  place_label TEXT
);

CREATE TABLE people (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  UNIQUE (library_id, normalized_name)
);

CREATE TABLE media_people (
  media_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
  person_id INTEGER NOT NULL REFERENCES people(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, person_id)
);

CREATE TABLE albums (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  name TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('takeout_folder','local')),
  folder_relative_path TEXT,
  description TEXT,
  cover_media_id INTEGER REFERENCES media_items(id) ON DELETE SET NULL,  -- F04: SET NULL
  created_at TEXT NOT NULL
);

CREATE TABLE media_albums (
  media_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
  album_id INTEGER NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, album_id)
);

CREATE TABLE media_metadata_raw (
  media_id INTEGER PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
  takeout_json TEXT,
  exif_json TEXT
);

CREATE TABLE duplicate_groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  match_type TEXT NOT NULL CHECK (match_type IN ('exact_hash','perceptual','same_json')),
  representative_media_id INTEGER REFERENCES media_items(id) ON DELETE SET NULL  -- F04: SET NULL
);

CREATE TABLE duplicate_group_members (
  group_id INTEGER NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
  media_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
  PRIMARY KEY (group_id, media_id)
);

CREATE TABLE user_notes (
  media_id INTEGER PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
  note TEXT,
  updated_at TEXT
);

-- F04: lixeira do app (AD-009). O JSON do Takeout vai junto com a mídia (AD-008).
CREATE TABLE trash_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  kind TEXT NOT NULL CHECK (kind IN ('file','folder')),
  media_id INTEGER REFERENCES media_items(id) ON DELETE SET NULL,
  original_relative_path TEXT NOT NULL,
  trash_relative_path TEXT NOT NULL UNIQUE,
  json_original_relative_path TEXT,
  json_trash_relative_path TEXT,
  trashed_at TEXT NOT NULL,
  purge_after TEXT GENERATED ALWAYS AS
    (strftime('%Y-%m-%dT%H:%M:%SZ', trashed_at, '+30 days')) STORED
);
CREATE INDEX idx_trash_purge ON trash_entries(purge_after);

CREATE TABLE indexing_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL REFERENCES libraries(id),
  started_at TEXT NOT NULL,
  finished_at TEXT,
  run_type TEXT NOT NULL CHECK (run_type IN ('full','incremental')),
  files_scanned INTEGER DEFAULT 0,
  files_indexed INTEGER DEFAULT 0,
  files_errored INTEGER DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running','completed','failed','cancelled'))
);

CREATE TABLE indexing_errors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id INTEGER NOT NULL REFERENCES indexing_runs(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  error_type TEXT NOT NULL CHECK (error_type IN (
    'corrupt_media','invalid_json','orphan_json','unreadable',
    'unreadable_directory','invalid_name','unknown_extension')),  -- F04: tipos do F02
  message TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT
);
