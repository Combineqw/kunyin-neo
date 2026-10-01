//! SQLite-backed local music library.
//!
//! This crate intentionally stores provider-specific song payloads as JSON.  That
//! keeps the Rust repository compatible with the existing Electron IPC DTOs while
//! allowing the database and playlist semantics to migrate independently.

use std::path::Path;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: i64 = 10;
pub const SYSTEM_KIND_TRIAL: &str = "trial";
pub const SYSTEM_KIND_FAVORITES: &str = "favorites";
pub const SYSTEM_TRIAL_NAME: &str = "试听列表";
pub const SYSTEM_FAVORITES_NAME: &str = "我的收藏";

#[derive(Debug)]
pub enum LibraryError {
    Sqlite(rusqlite::Error),
    InvalidSong(String),
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "sqlite error: {error}"),
            Self::InvalidSong(message) => write!(f, "invalid song: {message}"),
        }
    }
}

impl std::error::Error for LibraryError {}

impl From<rusqlite::Error> for LibraryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

pub type Result<T> = std::result::Result<T, LibraryError>;

/// A song row and its original provider payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongRecord {
    pub id: i64,
    pub source: String,
    pub song_json: String,
}

impl SongRecord {
    /// Construct a row from a JSON object while checking the id/source fields.
    /// The existing MusicItem uses numeric `id` and string `type` as its source.
    pub fn from_json(song_json: impl Into<String>) -> Result<Self> {
        let song_json = song_json.into();
        let value: serde_json::Value = serde_json::from_str(&song_json)
            .map_err(|error| LibraryError::InvalidSong(error.to_string()))?;
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| LibraryError::InvalidSong("missing numeric id".into()))?;
        let source = value
            .get("type")
            .or_else(|| value.get("source"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| LibraryError::InvalidSong("missing source/type".into()))?;
        Ok(Self {
            id,
            source: source.to_owned(),
            song_json,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub song_count: i64,
    pub cover_url: Option<String>,
    pub remote_source: Option<String>,
    pub remote_id: Option<String>,
    pub auto_refresh: bool,
    pub is_system: bool,
    pub system_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawPlaylistSong {
    pub song_json: String,
    pub added_at: i64,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistOptions {
    pub remote_source: Option<String>,
    pub remote_id: Option<String>,
    pub auto_refresh: bool,
}

impl Default for PlaylistOptions {
    fn default() -> Self {
        Self {
            remote_source: None,
            remote_id: None,
            auto_refresh: false,
        }
    }
}

/// Synchronous local-library repository.  Tauri commands can own one instance,
/// while the Electron adapter can use the same methods during shadow migration.
pub struct Library {
    connection: Connection,
}

impl Library {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Wrap an existing connection. Useful for migration tests and adapters that
    /// already own their SQLite connection.
    pub fn from_connection(connection: Connection) -> Result<Self> {
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        create_schema(&connection)?;
        migrate_schema(&connection)?;
        seed_system_playlists(&connection)?;
        Ok(Self { connection })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    pub fn system_playlist_id(&self, kind: &str) -> Result<Option<i64>> {
        Ok(self
            .connection
            .query_row(
                "SELECT playlist_id FROM playlists WHERE system_kind = ?1 LIMIT 1",
                [kind],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn trial_playlist_id(&self) -> Result<i64> {
        self.system_playlist_id(SYSTEM_KIND_TRIAL)?
            .ok_or_else(|| LibraryError::InvalidSong("trial playlist was not seeded".into()))
    }

    pub fn favorites_playlist_id(&self) -> Result<i64> {
        self.system_playlist_id(SYSTEM_KIND_FAVORITES)?
            .ok_or_else(|| LibraryError::InvalidSong("favorites playlist was not seeded".into()))
    }

    pub fn create_playlist(
        &self,
        name: &str,
        created_at: i64,
        options: &PlaylistOptions,
    ) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO playlists (name, created_at, remote_source, remote_id, auto_refresh, is_system, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, 0,
               (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM playlists WHERE is_system = 0))",
            params![
                name,
                created_at,
                options.remote_source,
                options.remote_id,
                options.auto_refresh as i64
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn rename_playlist(&self, playlist_id: i64, name: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE playlists SET name = ?1 WHERE playlist_id = ?2 AND is_system = 0",
            params![name, playlist_id],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, playlist_id: i64) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        let is_system: Option<i64> = tx
            .query_row(
                "SELECT is_system FROM playlists WHERE playlist_id = ?1",
                [playlist_id],
                |row| row.get(0),
            )
            .optional()?;
        if is_system == Some(0) {
            tx.execute(
                "DELETE FROM playlist_songs WHERE playlist_id = ?1",
                [playlist_id],
            )?;
            tx.execute(
                "DELETE FROM playlists WHERE playlist_id = ?1",
                [playlist_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_auto_refresh(&self, playlist_id: i64, enabled: bool) -> Result<()> {
        self.connection.execute(
            "UPDATE playlists SET auto_refresh = ?1 WHERE playlist_id = ?2",
            params![enabled as i64, playlist_id],
        )?;
        Ok(())
    }

    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        let mut statement = self.connection.prepare(
            "SELECT p.playlist_id, p.name, p.created_at, p.remote_source, p.remote_id,
                    p.auto_refresh, p.is_system, p.system_kind, COUNT(ps.song_id),
                    (SELECT s2.song_json FROM playlist_songs ps2
                     INNER JOIN songs s2 ON ps2.song_id = s2.song_id AND ps2.source = s2.source
                     WHERE ps2.playlist_id = p.playlist_id ORDER BY ps2.position ASC LIMIT 1)
             FROM playlists p LEFT JOIN playlist_songs ps ON p.playlist_id = ps.playlist_id
             GROUP BY p.playlist_id
             ORDER BY p.is_system DESC, p.sort_order ASC, p.created_at DESC",
        )?;
        let rows = statement.query_map([], playlist_from_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn get_playlist(&self, playlist_id: i64) -> Result<Option<Playlist>> {
        Ok(self
            .list_playlists()?
            .into_iter()
            .find(|playlist| playlist.id == playlist_id))
    }

    pub fn upsert_song(&self, song: &SongRecord) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO songs (song_id, source, song_json) VALUES (?1, ?2, ?3)",
            params![song.id, song.source, song.song_json],
        )?;
        index_song(&self.connection, song)?;
        Ok(())
    }

    /// Search indexed local/provider songs without parsing every JSON row in
    /// the host process. The query is treated as a sequence of literal terms
    /// joined with AND, so punctuation cannot turn into FTS operators.
    pub fn search_songs(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SongRecord>> {
        let match_query = literal_match_query(query);
        if match_query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let limit = i64::try_from(limit)
            .map_err(|_| LibraryError::InvalidSong("搜索结果上限过大".into()))?;
        let offset =
            i64::try_from(offset).map_err(|_| LibraryError::InvalidSong("搜索偏移过大".into()))?;

        // SQLite's default unicode61 tokenizer treats a contiguous CJK title as
        // one token, so a short Chinese query such as "江南" cannot match a
        // title containing "春日江南". Keep the indexed FTS path for the
        // common ASCII case and use a parameterized substring fallback for
        // non-ASCII or short terms. This keeps terms literal and avoids
        // exposing user input as SQL or FTS syntax.
        let terms = query
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .collect::<Vec<_>>();
        if terms
            .iter()
            .any(|term| term.chars().count() < 3 || !term.is_ascii())
        {
            return search_songs_like(&self.connection, &terms, limit, offset);
        }
        let mut statement = self.connection.prepare(
            "SELECT s.song_id, s.source, s.song_json
             FROM song_search idx
             INNER JOIN songs s ON idx.song_id = s.song_id AND idx.source = s.source
             WHERE song_search MATCH ?1
             ORDER BY bm25(song_search), s.song_id, s.source
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![match_query, limit, offset], |row| {
            Ok(SongRecord {
                id: row.get(0)?,
                source: row.get(1)?,
                song_json: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn add_to_playlist(
        &self,
        playlist_id: i64,
        song: &SongRecord,
        added_at: i64,
    ) -> Result<bool> {
        self.upsert_song(song)?;
        let position: i64 = self.connection.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_songs WHERE playlist_id = ?1",
            [playlist_id],
            |row| row.get(0),
        )?;
        let changed = self.connection.execute(
            "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_id, source, added_at, position)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![playlist_id, song.id, song.source, added_at, position],
        )?;
        Ok(changed > 0)
    }

    pub fn add_to_playlist_batch(
        &mut self,
        playlist_id: i64,
        songs: &[SongRecord],
        added_at: i64,
    ) -> Result<(usize, usize)> {
        if songs.is_empty() {
            return Ok((0, 0));
        }
        let tx = self.connection.transaction()?;
        let mut position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_songs WHERE playlist_id = ?1",
            [playlist_id],
            |row| row.get(0),
        )?;
        let mut added = 0;
        let mut skipped = 0;
        for song in songs {
            let changed = tx.execute(
                "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_id, source, added_at, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![playlist_id, song.id, song.source, added_at, position],
            )?;
            if changed > 0 {
                tx.execute(
                    "INSERT OR REPLACE INTO songs (song_id, source, song_json) VALUES (?1, ?2, ?3)",
                    params![song.id, song.source, song.song_json],
                )?;
                index_song(&tx, song)?;
                added += 1;
                position += 1;
            } else {
                skipped += 1;
            }
        }
        tx.commit()?;
        Ok((added, skipped))
    }

    pub fn remove_from_playlist(
        &self,
        playlist_id: i64,
        song_id: i64,
        source: &str,
    ) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM playlist_songs WHERE playlist_id = ?1 AND song_id = ?2 AND source = ?3",
            params![playlist_id, song_id, source],
        )? > 0)
    }

    pub fn query_playlist_songs(&self, playlist_id: i64) -> Result<Vec<SongRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT s.song_id, s.source, s.song_json FROM playlist_songs ps
             INNER JOIN songs s ON ps.song_id = s.song_id AND ps.source = s.source
             WHERE ps.playlist_id = ?1 ORDER BY ps.position ASC",
        )?;
        let rows = statement.query_map([playlist_id], |row| {
            Ok(SongRecord {
                id: row.get(0)?,
                source: row.get(1)?,
                song_json: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn query_playlist_songs_raw(&self, playlist_id: i64) -> Result<Vec<RawPlaylistSong>> {
        let mut statement = self.connection.prepare(
            "SELECT s.song_json, ps.added_at, ps.position FROM playlist_songs ps
             INNER JOIN songs s ON ps.song_id = s.song_id AND ps.source = s.source
             WHERE ps.playlist_id = ?1 ORDER BY ps.position ASC",
        )?;
        let rows = statement.query_map([playlist_id], |row| {
            Ok(RawPlaylistSong {
                song_json: row.get(0)?,
                added_at: row.get(1)?,
                position: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn replace_playlist_songs(
        &mut self,
        playlist_id: i64,
        songs: &[SongRecord],
        added_at: i64,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "DELETE FROM playlist_songs WHERE playlist_id = ?1",
            [playlist_id],
        )?;
        insert_songs(&tx, playlist_id, songs, added_at)?;
        tx.commit()?;
        Ok(())
    }

    pub fn move_song(
        &mut self,
        playlist_id: i64,
        song_id: i64,
        source: &str,
        target_position: i64,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        let mut entries: Vec<(i64, String)> = {
            let mut statement = tx.prepare(
                "SELECT song_id, source FROM playlist_songs WHERE playlist_id = ?1 ORDER BY position ASC",
            )?;
            let rows = statement.query_map([playlist_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        let Some(index) = entries
            .iter()
            .position(|entry| entry.0 == song_id && entry.1 == source)
        else {
            tx.commit()?;
            return Ok(());
        };
        let entry = entries.remove(index);
        let target = target_position.clamp(0, entries.len() as i64) as usize;
        entries.insert(target, entry);
        for (position, (id, entry_source)) in entries.iter().enumerate() {
            tx.execute(
                "UPDATE playlist_songs SET position = ?1 WHERE playlist_id = ?2 AND song_id = ?3 AND source = ?4",
                params![position as i64, playlist_id, id, entry_source],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_redirect(
        &self,
        song_id: i64,
        source: &str,
        target_song_json: &str,
        created_at: i64,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO song_redirects (song_id, source, target_song_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![song_id, source, target_song_json, created_at],
        )?;
        Ok(())
    }

    pub fn get_redirect(&self, song_id: i64, source: &str) -> Result<Option<SongRecord>> {
        let row = self
            .connection
            .query_row(
                "SELECT target_song_json FROM song_redirects WHERE song_id = ?1 AND source = ?2",
                params![song_id, source],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        row.map(SongRecord::from_json).transpose()
    }

    pub fn clear_redirect(&self, song_id: i64, source: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM song_redirects WHERE song_id = ?1 AND source = ?2",
            params![song_id, source],
        )? > 0)
    }
}

fn insert_songs(
    tx: &Transaction<'_>,
    playlist_id: i64,
    songs: &[SongRecord],
    added_at: i64,
) -> Result<()> {
    for (position, song) in songs.iter().enumerate() {
        tx.execute(
            "INSERT OR REPLACE INTO songs (song_id, source, song_json) VALUES (?1, ?2, ?3)",
            params![song.id, song.source, song.song_json],
        )?;
        index_song(tx, song)?;
        tx.execute(
            "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_id, source, added_at, position)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![playlist_id, song.id, song.source, added_at, position as i64],
        )?;
    }
    Ok(())
}

fn playlist_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Playlist> {
    let first_song_json: Option<String> = row.get(9)?;
    let cover_url = first_song_json.and_then(|json| {
        serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|value| {
                value
                    .get("cover")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
    });
    Ok(Playlist {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        remote_source: row.get(3)?,
        remote_id: row.get(4)?,
        auto_refresh: row.get::<_, i64>(5)? != 0,
        is_system: row.get::<_, i64>(6)? != 0,
        system_kind: row.get(7)?,
        song_count: row.get(8)?,
        cover_url,
    })
}

fn create_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS songs (
           song_id INTEGER NOT NULL, source TEXT NOT NULL, song_json TEXT NOT NULL,
           PRIMARY KEY(song_id, source)
         );
         CREATE TABLE IF NOT EXISTS playlists (
           playlist_id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
           created_at INTEGER NOT NULL, remote_source TEXT, remote_id TEXT,
           auto_refresh INTEGER NOT NULL DEFAULT 0, is_system INTEGER NOT NULL DEFAULT 0,
           system_kind TEXT, sort_order INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS playlist_songs (
           playlist_id INTEGER NOT NULL, song_id INTEGER NOT NULL, source TEXT NOT NULL,
           added_at INTEGER NOT NULL, position INTEGER NOT NULL DEFAULT 0,
           PRIMARY KEY(playlist_id, song_id, source)
         );
         CREATE TABLE IF NOT EXISTS song_redirects (
           song_id INTEGER NOT NULL, source TEXT NOT NULL, target_song_json TEXT NOT NULL,
           created_at INTEGER NOT NULL, PRIMARY KEY(song_id, source)
         );
         CREATE TABLE IF NOT EXISTS play_events (
           id INTEGER PRIMARY KEY AUTOINCREMENT, song_id INTEGER NOT NULL, source TEXT NOT NULL,
           played_at INTEGER NOT NULL, duration_ms INTEGER NOT NULL, played_ms INTEGER NOT NULL,
           completion REAL NOT NULL DEFAULT 0, skipped INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS idx_play_events_song ON play_events(song_id, source);
         CREATE INDEX IF NOT EXISTS idx_play_events_time ON play_events(played_at);
         CREATE TABLE IF NOT EXISTS song_metadata (
           song_id INTEGER NOT NULL, source TEXT NOT NULL, tags_json TEXT NOT NULL DEFAULT '[]',
           genre TEXT, bpm REAL, updated_at INTEGER NOT NULL, PRIMARY KEY(song_id, source)
         );
         CREATE INDEX IF NOT EXISTS idx_song_metadata_genre ON song_metadata(genre);
         CREATE TABLE IF NOT EXISTS recommendation_cache (
           cache_key TEXT PRIMARY KEY, payload_json TEXT NOT NULL, updated_at INTEGER NOT NULL
         );
         CREATE VIRTUAL TABLE IF NOT EXISTS song_search USING fts5(
           song_id UNINDEXED, source UNINDEXED, title, artist, album
         );",
    )?;
    Ok(())
}

fn migrate_schema(connection: &Connection) -> Result<()> {
    let has_sort_order: bool = connection
        .prepare("PRAGMA table_info(playlists)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == "sort_order");
    if !has_sort_order {
        connection.execute(
            "ALTER TABLE playlists ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        let mut ids = connection
            .prepare(
                "SELECT playlist_id FROM playlists WHERE is_system = 0 ORDER BY created_at DESC",
            )?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let tx = connection.unchecked_transaction()?;
        for (position, id) in ids.drain(..).enumerate() {
            tx.execute(
                "UPDATE playlists SET sort_order = ?1 WHERE playlist_id = ?2",
                params![position as i64, id],
            )?;
        }
        tx.commit()?;
    }
    let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current < SCHEMA_VERSION {
        rebuild_search_index(connection)?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

/// Keep the FTS index in sync with the canonical JSON song table. The index is
/// intentionally denormalized and can always be rebuilt during migration.
fn index_song(connection: &Connection, song: &SongRecord) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(&song.song_json)
        .map_err(|error| LibraryError::InvalidSong(error.to_string()))?;
    let text = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    };
    connection.execute(
        "DELETE FROM song_search WHERE song_id = ?1 AND source = ?2",
        params![song.id, song.source],
    )?;
    connection.execute(
        "INSERT INTO song_search (song_id, source, title, artist, album)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            song.id,
            song.source,
            text("title"),
            text("artist"),
            text("album")
        ],
    )?;
    Ok(())
}

fn rebuild_search_index(connection: &Connection) -> Result<()> {
    connection.execute("DELETE FROM song_search", [])?;
    let songs = {
        let mut statement = connection.prepare("SELECT song_id, source, song_json FROM songs")?;
        let rows = statement.query_map([], |row| {
            Ok(SongRecord {
                id: row.get(0)?,
                source: row.get(1)?,
                song_json: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for song in songs {
        index_song(connection, &song)?;
    }
    Ok(())
}

fn literal_match_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn search_songs_like(
    connection: &Connection,
    terms: &[&str],
    limit: i64,
    offset: i64,
) -> Result<Vec<SongRecord>> {
    let clauses = terms
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let title = index * 3 + 1;
            let artist = title + 1;
            let album = title + 2;
            format!(
                "(idx.title LIKE ?{title} ESCAPE '\\' OR idx.artist LIKE ?{artist} ESCAPE '\\' OR idx.album LIKE ?{album} ESCAPE '\\')"
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "SELECT s.song_id, s.source, s.song_json
         FROM song_search idx
         INNER JOIN songs s ON idx.song_id = s.song_id AND idx.source = s.source
         WHERE {clauses}
         ORDER BY s.song_id, s.source
         LIMIT {limit} OFFSET {offset}"
    );
    let values = terms
        .iter()
        .flat_map(|term| {
            let value = format!("%{}%", escape_like_term(term));
            [value.clone(), value.clone(), value]
        })
        .collect::<Vec<_>>();
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        Ok(SongRecord {
            id: row.get(0)?,
            source: row.get(1)?,
            song_json: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn escape_like_term(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn seed_system_playlists(connection: &Connection) -> Result<()> {
    let insert = |name: &str, created_at: i64, kind: &str| -> Result<()> {
        let exists: Option<i64> = connection
            .query_row(
                "SELECT playlist_id FROM playlists WHERE system_kind = ?1 LIMIT 1",
                [kind],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            connection.execute(
                "INSERT INTO playlists (name, created_at, auto_refresh, is_system, system_kind)
                 VALUES (?1, ?2, 0, 1, ?3)",
                params![name, created_at, kind],
            )?;
        }
        Ok(())
    };
    insert(SYSTEM_TRIAL_NAME, 2, SYSTEM_KIND_TRIAL)?;
    insert(SYSTEM_FAVORITES_NAME, 1, SYSTEM_KIND_FAVORITES)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn song(id: i64, title: &str) -> SongRecord {
        SongRecord::from_json(format!(
            r#"{{"id":{id},"type":"local","title":"{title}","cover":"cover-{id}"}}"#
        ))
        .unwrap()
    }

    #[test]
    fn creates_v9_schema_and_seeds_system_playlists() {
        let library = Library::open_in_memory().unwrap();
        assert_eq!(library.schema_version().unwrap(), SCHEMA_VERSION);
        let playlists = library.list_playlists().unwrap();
        assert_eq!(playlists.len(), 2);
        assert_eq!(playlists[0].system_kind.as_deref(), Some(SYSTEM_KIND_TRIAL));
        assert_eq!(
            playlists[1].system_kind.as_deref(),
            Some(SYSTEM_KIND_FAVORITES)
        );
        assert!(playlists.iter().all(|playlist| playlist.is_system));
    }

    #[test]
    fn fts_search_matches_literal_song_fields_and_paginates() {
        let library = Library::open_in_memory().unwrap();
        library
            .upsert_song(&song(1, "春日江南"))
            .expect("index first song");
        library
            .upsert_song(
                &SongRecord::from_json(
                    r#"{"id":2,"type":"local","title":"秋夜","artist":"江南乐队","album":"现场"}"#,
                )
                .unwrap(),
            )
            .expect("index second song");

        let matches = library.search_songs("江南", 10, 0).expect("search");
        assert_eq!(
            matches.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        let page = library.search_songs("江南", 1, 1).expect("page");
        assert_eq!(page.len(), 1);
        assert_eq!(library.search_songs("title:江南", 10, 0).unwrap().len(), 0);
        assert!(library.search_songs("   ", 10, 0).unwrap().is_empty());
    }

    #[test]
    fn playlist_crud_preserves_order_and_duplicate_semantics() {
        let mut library = Library::open_in_memory().unwrap();
        let playlist = library
            .create_playlist("Mix", 100, &PlaylistOptions::default())
            .unwrap();
        let first = song(1, "First");
        let second = song(2, "Second");
        assert!(library.add_to_playlist(playlist, &first, 101).unwrap());
        assert!(library.add_to_playlist(playlist, &second, 102).unwrap());
        assert!(!library.add_to_playlist(playlist, &first, 103).unwrap());
        assert_eq!(
            library.query_playlist_songs(playlist).unwrap(),
            vec![first.clone(), second.clone()]
        );
        library.move_song(playlist, 2, "local", 0).unwrap();
        assert_eq!(
            library.query_playlist_songs(playlist).unwrap(),
            vec![second, first]
        );
        assert!(library.remove_from_playlist(playlist, 2, "local").unwrap());
        assert_eq!(library.query_playlist_songs(playlist).unwrap().len(), 1);
        library.rename_playlist(playlist, "Renamed").unwrap();
        assert_eq!(
            library.get_playlist(playlist).unwrap().unwrap().name,
            "Renamed"
        );
    }

    #[test]
    fn batch_replace_and_redirect_are_transactional() {
        let mut library = Library::open_in_memory().unwrap();
        let playlist = library
            .create_playlist("Batch", 1, &PlaylistOptions::default())
            .unwrap();
        let songs = vec![song(10, "A"), song(11, "B")];
        assert_eq!(
            library.add_to_playlist_batch(playlist, &songs, 2).unwrap(),
            (2, 0)
        );
        assert_eq!(
            library.add_to_playlist_batch(playlist, &songs, 3).unwrap(),
            (0, 2)
        );
        library
            .replace_playlist_songs(playlist, &[songs[1].clone()], 4)
            .unwrap();
        assert_eq!(
            library.query_playlist_songs(playlist).unwrap(),
            vec![songs[1].clone()]
        );
        library
            .set_redirect(10, "local", &songs[1].song_json, 5)
            .unwrap();
        assert_eq!(
            library.get_redirect(10, "local").unwrap(),
            Some(songs[1].clone())
        );
        assert!(library.clear_redirect(10, "local").unwrap());
        assert_eq!(library.get_redirect(10, "local").unwrap(), None);
    }

    #[test]
    fn v6_database_gets_sort_order_and_v9_tables() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE songs (song_id INTEGER NOT NULL, source TEXT NOT NULL, song_json TEXT NOT NULL, PRIMARY KEY(song_id, source));
             CREATE TABLE playlists (playlist_id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at INTEGER NOT NULL,
               remote_source TEXT, remote_id TEXT, auto_refresh INTEGER NOT NULL DEFAULT 0, is_system INTEGER NOT NULL DEFAULT 0, system_kind TEXT);
             CREATE TABLE playlist_songs (playlist_id INTEGER NOT NULL, song_id INTEGER NOT NULL, source TEXT NOT NULL, added_at INTEGER NOT NULL, position INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(playlist_id, song_id, source));
             CREATE TABLE song_redirects (song_id INTEGER NOT NULL, source TEXT NOT NULL, target_song_json TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(song_id, source));
             PRAGMA user_version = 6;
             INSERT INTO playlists (name, created_at, is_system) VALUES ('Old', 10, 0);",
        )
        .unwrap();
        let library = Library::from_connection(connection).unwrap();
        assert_eq!(library.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(library.get_playlist(1).unwrap().is_some());
        let has_sort_order: i64 = library
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('playlists') WHERE name='sort_order'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_sort_order, 1);
        assert_eq!(library.list_playlists().unwrap().len(), 3);
    }
}
