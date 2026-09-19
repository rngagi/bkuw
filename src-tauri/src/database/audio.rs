//! Project-local audio: conversion, integrity, ownership, and transactional attachment.
use super::*;
use crate::domain::{
    AudioAttachment, AudioContent, AudioMutation, AudioOwner, ImportAudioRequest,
    RemoveAudioRequest,
};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::Instant,
};
use tempfile::{NamedTempFile, TempDir};
use wait_timeout::ChildExt;

const MAX_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DURATION: f64 = 1800.0;
const MAX_MP3_BYTES: u64 = 16 * 1024 * 1024;

fn invalid() -> AppError {
    AppError::new(
        "audio_invalid",
        "The audio is unsupported, damaged, or empty.",
    )
}
fn limit() -> AppError {
    AppError::new("audio_limit", "Audio exceeds the size or duration limit.")
}

pub(crate) struct AudioTools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}
impl AudioTools {
    pub(crate) fn bundled(root: &Path) -> AppResult<Self> {
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("manifest.json")).map_err(|_| {
                AppError::new("audio_tools", "The bundled audio tools are unavailable.")
            })?)
            .map_err(|_| invalid())?;
        let executable = |name: &str| -> AppResult<PathBuf> {
            let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            let path = root.join(&filename);
            let bytes = fs::read(&path).map_err(|_| {
                AppError::new("audio_tools", "The bundled audio tools are unavailable.")
            })?;
            if manifest["sha256"][&filename].as_str()
                != Some(hex::encode(Sha256::digest(bytes)).as_str())
            {
                return Err(AppError::new(
                    "audio_tools",
                    "The bundled audio tools failed verification.",
                ));
            }
            Ok(path)
        };
        Ok(Self {
            ffmpeg: executable("ffmpeg")?,
            ffprobe: executable("ffprobe")?,
        })
    }
}

pub(crate) struct PreparedAudio {
    _directory: TempDir,
    path: PathBuf,
    original_filename: String,
    duration_ms: u64,
}

// Use regular temporary files for output so a full pipe cannot deadlock a child.
fn run(command: &mut Command, deadline: Instant) -> AppResult<Vec<u8>> {
    let output = NamedTempFile::new()?;
    command
        .stdin(Stdio::null())
        .stdout(output.reopen()?)
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| AppError::new("audio_tools", "The audio tool could not start."))?;
    let status = child.wait_timeout(deadline.saturating_duration_since(Instant::now()));
    match status {
        Ok(Some(status)) if status.success() => {}
        Ok(Some(_)) => return Err(invalid()),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "audio_timeout",
                "Audio conversion timed out.",
            ));
        }
    }
    let mut bytes = Vec::new();
    output.reopen()?.take(1024 * 1024).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn probe(tools: &AudioTools, path: &Path, deadline: Instant) -> AppResult<serde_json::Value> {
    let bytes = run(
        Command::new(&tools.ffprobe)
            .args([
                "-v",
                "error",
                "-protocol_whitelist",
                "file,pipe",
                "-show_entries",
                "stream=codec_type,codec_name,sample_rate,channels,bit_rate:format=duration",
                "-of",
                "json",
            ])
            .arg(path),
        deadline,
    )?;
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}
fn duration(value: &serde_json::Value, maximum: f64) -> AppResult<f64> {
    let seconds = value["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(invalid)?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(invalid());
    }
    if seconds > maximum {
        return Err(limit());
    }
    Ok(seconds)
}

pub(crate) fn prepare(tools: &AudioTools, source: &Path) -> AppResult<PreparedAudio> {
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ![
        "wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "aif", "aiff", "aifc",
    ]
    .contains(&extension.as_str())
    {
        return Err(invalid());
    }
    if !fs::metadata(source).map_err(|_| invalid())?.is_file() {
        return Err(invalid());
    }
    let mut input = File::open(source).map_err(|_| invalid())?;
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(invalid());
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(limit());
    }
    let directory = tempfile::tempdir()?;
    let copied = directory.path().join(format!("source.{extension}"));
    let mut copy = File::create(&copied)?;
    let copied_size = std::io::copy(
        &mut Read::by_ref(&mut input).take(MAX_SOURCE_BYTES + 1),
        &mut copy,
    )?;
    if copied_size > MAX_SOURCE_BYTES {
        return Err(limit());
    }
    drop(copy);
    let deadline = Instant::now() + Duration::from_secs(300);
    let info = probe(tools, &copied, deadline)?;
    duration(&info, MAX_DURATION)?;
    let streams = info["streams"].as_array().ok_or_else(invalid)?;
    let audio = streams
        .iter()
        .filter(|s| s["codec_type"] == "audio")
        .collect::<Vec<_>>();
    if audio.len() != 1 {
        return Err(invalid());
    }
    let codec = audio[0]["codec_name"].as_str().ok_or_else(invalid)?;
    if !codec.starts_with("pcm_")
        && !["mp3", "aac", "alac", "flac", "vorbis", "opus"].contains(&codec)
    {
        return Err(invalid());
    }
    let path = directory.path().join("converted.mp3");
    run(
        Command::new(&tools.ffmpeg)
            .args([
                "-nostdin",
                "-v",
                "error",
                "-xerror",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(&copied)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-sn",
                "-dn",
                "-map_metadata",
                "-1",
                "-c:a",
                "libmp3lame",
                "-b:a",
                "64k",
                "-ac",
                "1",
                "-ar",
                "44100",
                "-t",
                "1800.1",
                "-f",
                "mp3",
            ])
            .arg(&path),
        deadline,
    )?;
    let encoded = probe(tools, &path, deadline)?;
    // MP3 frame padding adds at most a few frames; never accept the 1800.1s safety cut.
    let seconds = duration(&encoded, MAX_DURATION + 0.1)?;
    let stream = &encoded["streams"][0];
    if stream["codec_name"] != "mp3"
        || stream["sample_rate"] != "44100"
        || stream["channels"] != 1
        || stream["bit_rate"] != "64000"
    {
        return Err(invalid());
    }
    if fs::metadata(&path)?.len() > MAX_MP3_BYTES {
        return Err(limit());
    }
    let original_filename = normalize_text(
        source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(invalid)?,
    );
    Ok(PreparedAudio {
        _directory: directory,
        path,
        original_filename,
        duration_ms: (seconds * 1000.0).round() as u64,
    })
}

fn owner_parts(owner: &AudioOwner) -> (Option<&str>, Option<&str>) {
    match owner {
        AudioOwner::Sense(id) => (Some(id), None),
        AudioOwner::Example(id) => (None, Some(id)),
    }
}
fn owner_entry(connection: &Connection, owner: &AudioOwner) -> AppResult<String> {
    let (sql, id) = match owner {
        AudioOwner::Sense(id) => (
            "SELECT s.entry_id FROM senses s JOIN lexical_entries e ON e.id=s.entry_id WHERE s.id=?1 AND e.deleted_at IS NULL",
            id,
        ),
        AudioOwner::Example(id) => (
            "SELECT s.entry_id FROM examples x JOIN senses s ON s.id=x.sense_id JOIN lexical_entries e ON e.id=s.entry_id WHERE x.id=?1 AND e.deleted_at IS NULL",
            id,
        ),
    };
    connection
        .query_row(sql, params![id], |row| row.get(0))
        .optional()?
        .ok_or_else(|| AppError::new("audio_not_found", "The audio owner was not found."))
}

impl ProjectSession {
    pub(crate) fn audio_import_token(&self, request: &ImportAudioRequest) -> AppResult<String> {
        if owner_entry(&self.connection, &request.owner)? != request.entry_id {
            return Err(invalid());
        }
        let entry = self.load_entry(&request.entry_id)?;
        if entry.revision != request.expected_revision {
            return Err(AppError::new(
                "revision_conflict",
                "The entry changed. Retry the audio operation.",
            ));
        }
        Ok(self.session_token.clone())
    }
    pub fn list_audio(&self, owner: &AudioOwner) -> AppResult<Vec<AudioAttachment>> {
        owner_entry(&self.connection, owner)?;
        let (sense, example) = owner_parts(owner);
        let mut statement = self.connection.prepare("SELECT id, original_filename, duration_ms, byte_size, sort_order, created_at FROM audio_attachments WHERE sense_id=?1 OR example_id=?2 ORDER BY sort_order, id")?;
        let rows = statement.query_map(params![sense, example], |row| {
            Ok(AudioAttachment {
                id: row.get(0)?,
                owner: owner.clone(),
                original_filename: row.get(1)?,
                duration_ms: row.get(2)?,
                byte_size: row.get(3)?,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
    pub(crate) fn attach_audio(
        &mut self,
        request: ImportAudioRequest,
        token: &str,
        prepared: PreparedAudio,
    ) -> AppResult<AudioMutation> {
        if token != self.session_token {
            return Err(AppError::new(
                "audio_stale",
                "The project changed during conversion.",
            ));
        }
        self.audio_import_token(&request)?;
        let id = new_id();
        let relative_path = format!("media/audio/{id}.mp3");
        let destination = self.audio_path(&relative_path)?;
        let bytes = fs::read(&prepared.path)?;
        let hash = hex::encode(Sha256::digest(&bytes));
        let mut temporary = NamedTempFile::new_in(destination.parent().ok_or_else(invalid)?)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        let timestamp = now();
        let (sense, example) = owner_parts(&request.owner);
        let transaction = self.connection.transaction()?;
        let updated = transaction.execute("UPDATE lexical_entries SET revision=revision+1, updated_at=?1 WHERE id=?2 AND revision=?3 AND deleted_at IS NULL", params![timestamp, request.entry_id, request.expected_revision])?;
        if updated == 0 {
            return Err(revision_or_not_found(&transaction, &request.entry_id)?);
        }
        let sort_order: i64 = transaction.query_row("SELECT COALESCE(MAX(sort_order),-1)+1 FROM audio_attachments WHERE sense_id=?1 OR example_id=?2", params![sense, example], |row| row.get(0))?;
        transaction.execute("INSERT INTO audio_attachments (id,sense_id,example_id,relative_path,original_filename,duration_ms,byte_size,sha256,sort_order,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![id,sense,example,relative_path,prepared.original_filename,prepared.duration_ms,bytes.len() as i64,hash,sort_order,timestamp])?;
        temporary
            .persist_noclobber(&destination)
            .map_err(|e| AppError::from(e.error))?;
        if let Err(error) = transaction.commit() {
            let _ = fs::remove_file(destination);
            return Err(error.into());
        }
        Ok(AudioMutation {
            entry: self.load_entry(&request.entry_id)?,
            audio: Some(AudioAttachment {
                id,
                owner: request.owner,
                original_filename: prepared.original_filename,
                duration_ms: prepared.duration_ms,
                byte_size: bytes.len() as u64,
                sort_order,
                created_at: timestamp,
            }),
        })
    }
    pub fn load_audio(&self, id: &str) -> AppResult<AudioContent> {
        let (relative, hash, expected_size) = self.connection.query_row("SELECT a.relative_path,a.sha256,a.byte_size FROM audio_attachments a LEFT JOIN examples x ON x.id=a.example_id JOIN senses s ON s.id=COALESCE(a.sense_id,x.sense_id) JOIN lexical_entries e ON e.id=s.entry_id WHERE a.id=?1 AND e.deleted_at IS NULL", params![id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,u64>(2)?))).optional()?.ok_or_else(|| AppError::new("audio_not_found", "The audio was not found."))?;
        let path = self.audio_path(&relative)?;
        let file = File::open(path).map_err(|_| invalid())?;
        if expected_size > MAX_MP3_BYTES || file.metadata()?.len() != expected_size {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take(MAX_MP3_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected_size || hex::encode(Sha256::digest(&bytes)) != hash {
            return Err(invalid());
        }
        Ok(AudioContent {
            mime_type: "audio/mpeg".into(),
            data_base64: BASE64.encode(bytes),
        })
    }
    pub fn remove_audio(&mut self, request: RemoveAudioRequest) -> AppResult<AudioMutation> {
        let relative: String = self.connection.query_row("SELECT a.relative_path FROM audio_attachments a LEFT JOIN examples x ON x.id=a.example_id JOIN senses s ON s.id=COALESCE(a.sense_id,x.sense_id) WHERE a.id=?1 AND s.entry_id=?2", params![request.audio_id,request.entry_id], |row| row.get(0)).optional()?.ok_or_else(|| AppError::new("audio_not_found", "The audio was not found."))?;
        let path = self.audio_path(&relative)?;
        let transaction = self.connection.transaction()?;
        if transaction.execute("UPDATE lexical_entries SET revision=revision+1, updated_at=?1 WHERE id=?2 AND revision=?3 AND deleted_at IS NULL", params![now(),request.entry_id,request.expected_revision])? == 0 { return Err(revision_or_not_found(&transaction, &request.entry_id)?); }
        transaction.execute(
            "DELETE FROM audio_attachments WHERE id=?1",
            params![request.audio_id],
        )?;
        transaction.commit()?;
        let _ = fs::remove_file(path);
        Ok(AudioMutation {
            entry: self.load_entry(&request.entry_id)?,
            audio: None,
        })
    }
    fn audio_path(&self, relative: &str) -> AppResult<PathBuf> {
        let filename = relative.strip_prefix("media/audio/").ok_or_else(invalid)?;
        let id = filename.strip_suffix(".mp3").ok_or_else(invalid)?;
        if Uuid::parse_str(id).is_err() || filename.contains(['/', '\\']) {
            return Err(invalid());
        }
        let mut directory = self.root.clone();
        for component in ["media", "audio"] {
            directory.push(component);
            if directory.exists() {
                let metadata = fs::symlink_metadata(&directory)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(invalid());
                }
            } else {
                fs::create_dir(&directory)?;
            }
        }
        let path = directory.join(filename);
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(invalid());
        }
        Ok(path)
    }
    pub(super) fn entry_audio_paths(&self, entry: &str) -> AppResult<HashSet<String>> {
        let mut statement = self.connection.prepare("SELECT a.relative_path FROM audio_attachments a LEFT JOIN examples x ON x.id=a.example_id JOIN senses s ON s.id=COALESCE(a.sense_id,x.sense_id) WHERE s.entry_id=?1")?;
        let rows = statement.query_map(params![entry], |row| row.get(0))?;
        rows.collect::<Result<HashSet<_>, _>>().map_err(Into::into)
    }
    pub(super) fn clean_removed_audio(
        &self,
        previous: &HashSet<String>,
        entry: &str,
    ) -> AppResult<()> {
        let current = self.entry_audio_paths(entry)?;
        for relative in previous.difference(&current) {
            if let Ok(path) = self.audio_path(relative) {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod tests {
    use super::*;
    use crate::domain::{DeleteEntryRequest, Example};

    fn tools() -> AudioTools {
        AudioTools::bundled(&Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/audio"))
            .expect("run pnpm audio:prepare before Rust tests")
    }
    fn fixture(extension: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/audio/tone.{extension}"))
    }
    fn session() -> (TempDir, ProjectSession, crate::domain::LexicalEntry) {
        let directory = tempfile::tempdir().unwrap();
        let mut session = ProjectSession::create(CreateProjectRequest {
            parent_dir: directory.path().to_string_lossy().into_owned(),
            name: "Audio".into(),
            language_name: None,
            language_code: None,
        })
        .unwrap();
        let mut entry = session.create_entry().unwrap();
        entry.senses.push(Sense {
            id: new_id(),
            gloss: Some("voice".into()),
            definition: None,
            part_of_speech: None,
            semantic_domain: None,
            sort_order: 0,
            examples: vec![Example {
                id: new_id(),
                translation: None,
                notes: None,
                sort_order: 0,
                forms: vec![],
            }],
        });
        let entry = session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .unwrap();
        (directory, session, entry)
    }
    fn attach(
        session: &mut ProjectSession,
        entry: &crate::domain::LexicalEntry,
        owner: AudioOwner,
    ) -> AudioMutation {
        let request = ImportAudioRequest {
            entry_id: entry.id.clone(),
            owner,
            expected_revision: entry.revision,
            source_path: fixture("wav").to_string_lossy().into_owned(),
        };
        let token = session.audio_import_token(&request).unwrap();
        let prepared = prepare(&tools(), Path::new(&request.source_path)).unwrap();
        session.attach_audio(request, &token, prepared).unwrap()
    }
    #[test]
    fn supported_formats_convert_to_verified_mono_64k_mp3_without_changing_source() {
        let tools = tools();
        for extension in ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "aiff"] {
            let source = fixture(extension);
            let original = fs::read(&source).unwrap();
            let converted =
                prepare(&tools, &source).unwrap_or_else(|error| panic!("{extension}: {error:?}"));
            assert!(converted.duration_ms >= 200 && converted.duration_ms < 500);
            assert!(fs::metadata(&converted.path).unwrap().len() < 6000);
            assert_eq!(fs::read(&source).unwrap(), original);
        }
    }
    #[test]
    fn attachments_survive_autosave_soft_delete_reopen_and_project_move() {
        let (directory, mut session, entry) = session();
        let sense = AudioOwner::Sense(entry.senses[0].id.clone());
        let example = AudioOwner::Example(entry.senses[0].examples[0].id.clone());
        let first = attach(&mut session, &entry, sense.clone());
        let second = attach(&mut session, &first.entry, example.clone());
        let third = attach(&mut session, &second.entry, example.clone());
        let first_id = first.audio.unwrap().id;
        let second_id = second.audio.unwrap().id;
        let third_id = third.audio.unwrap().id;
        let mut entry = third.entry;
        entry.senses[0].examples[0].translation = Some("edited".into());
        let entry = session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .unwrap();
        assert_eq!(session.list_audio(&example).unwrap().len(), 2);
        session
            .delete_entry(DeleteEntryRequest {
                id: entry.id.clone(),
                expected_revision: entry.revision,
            })
            .unwrap();
        assert!(session.load_audio(&second_id).is_err());
        let restored = session.restore_entry(&entry.id).unwrap();
        assert!(session.load_audio(&first_id).is_ok());
        let old_root = session.root.clone();
        session.close().unwrap();
        let moved = directory.path().join("Moved.bkuw");
        fs::rename(old_root, &moved).unwrap();
        let mut session = ProjectSession::open(&moved).unwrap();
        assert!(session.load_audio(&third_id).is_ok());
        let removed = session
            .remove_audio(RemoveAudioRequest {
                entry_id: entry.id.clone(),
                audio_id: second_id.clone(),
                expected_revision: restored.revision,
            })
            .unwrap();
        assert!(!moved.join(format!("media/audio/{second_id}.mp3")).exists());
        let mut entry = removed.entry;
        entry.senses[0].examples.clear();
        let entry = session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .unwrap();
        assert!(!moved.join(format!("media/audio/{third_id}.mp3")).exists());
        assert!(session.load_audio(&first_id).is_ok());
        let mut entry = entry;
        entry.senses.clear();
        session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .unwrap();
        assert!(!moved.join(format!("media/audio/{first_id}.mp3")).exists());
    }
    #[test]
    fn rejects_stale_owner_revision_and_corrupt_project_media() {
        let (_directory, mut session, entry) = session();
        let owner = AudioOwner::Sense(entry.senses[0].id.clone());
        let request = ImportAudioRequest {
            entry_id: entry.id.clone(),
            owner: owner.clone(),
            expected_revision: entry.revision,
            source_path: fixture("wav").to_string_lossy().into_owned(),
        };
        let token = session.audio_import_token(&request).unwrap();
        let prepared = prepare(&tools(), &fixture("wav")).unwrap();
        assert_eq!(
            session
                .attach_audio(request.clone(), "previous-session", prepared)
                .err()
                .unwrap()
                .code,
            "audio_stale"
        );
        let result = attach(&mut session, &entry, owner.clone());
        let prepared = prepare(&tools(), &fixture("wav")).unwrap();
        assert_eq!(
            session
                .attach_audio(request, &token, prepared)
                .err()
                .unwrap()
                .code,
            "revision_conflict"
        );
        assert_eq!(session.list_audio(&owner).unwrap().len(), 1);
        let audio = result.audio.unwrap();
        let path = session.root.join(format!("media/audio/{}.mp3", audio.id));
        let mut bytes = fs::read(&path).unwrap();
        bytes[30] ^= 0xff;
        fs::write(&path, bytes).unwrap();
        assert_eq!(
            session.load_audio(&audio.id).err().unwrap().code,
            "audio_invalid"
        );
    }
    #[test]
    fn unicode_filename_is_nfc_and_limits_and_invalid_sources_fail() {
        let tools = tools();
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("語音-e\u{301}.WAV");
        fs::copy(fixture("wav"), &source).unwrap();
        let prepared = prepare(&tools, &source).unwrap();
        assert_eq!(prepared.original_filename, "語音-é.WAV");
        fs::write(&source, b"not audio").unwrap();
        assert_eq!(
            prepare(&tools, &source).err().unwrap().code,
            "audio_invalid"
        );
        let oversized = File::create(&source).unwrap();
        oversized.set_len(MAX_SOURCE_BYTES + 1).unwrap();
        drop(oversized);
        assert_eq!(prepare(&tools, &source).err().unwrap().code, "audio_limit");
        // 31 minutes of 8 kHz mono PCM; a sparse file exercises duration validation.
        let length = 31_u32 * 60 * 8000 * 2;
        let mut wav = File::create(&source).unwrap();
        wav.write_all(b"RIFF").unwrap();
        wav.write_all(&(length + 36).to_le_bytes()).unwrap();
        wav.write_all(b"WAVEfmt ").unwrap();
        wav.write_all(&16_u32.to_le_bytes()).unwrap();
        wav.write_all(&1_u16.to_le_bytes()).unwrap();
        wav.write_all(&1_u16.to_le_bytes()).unwrap();
        wav.write_all(&8000_u32.to_le_bytes()).unwrap();
        wav.write_all(&16000_u32.to_le_bytes()).unwrap();
        wav.write_all(&2_u16.to_le_bytes()).unwrap();
        wav.write_all(&16_u16.to_le_bytes()).unwrap();
        wav.write_all(b"data").unwrap();
        wav.write_all(&length.to_le_bytes()).unwrap();
        wav.set_len(u64::from(length) + 44).unwrap();
        drop(wav);
        assert_eq!(prepare(&tools, &source).err().unwrap().code, "audio_limit");
    }
    #[test]
    fn schema_six_migration_backs_up_without_losing_entries() {
        let (_directory, session, entry) = session();
        let root = session.root.clone();
        session.close().unwrap();
        let connection = Connection::open(root.join("project.sqlite")).unwrap();
        connection
            .execute_batch(
                "DROP TABLE audio_attachments; DELETE FROM schema_migrations WHERE version=7;",
            )
            .unwrap();
        drop(connection);
        let session = ProjectSession::open(&root).unwrap();
        assert_eq!(session.load_entry(&entry.id).unwrap(), entry);
        assert!(
            session
                .list_audio(&AudioOwner::Sense(entry.senses[0].id.clone()))
                .unwrap()
                .is_empty()
        );
        let backup = fs::read_dir(root.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let backup = Connection::open(backup).unwrap();
        let version: i64 = backup
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 6);
    }
    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_audio_files_and_directories() {
        let (directory, mut session, entry) = session();
        let owner = AudioOwner::Sense(entry.senses[0].id.clone());
        let result = attach(&mut session, &entry, owner);
        let id = result.audio.unwrap().id;
        let path = session.root.join(format!("media/audio/{id}.mp3"));
        let outside = directory.path().join("outside.mp3");
        fs::rename(&path, &outside).unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(session.load_audio(&id).is_err());
        fs::remove_file(&path).unwrap();
        fs::remove_dir(session.root.join("media/audio")).unwrap();
        std::os::unix::fs::symlink(directory.path(), session.root.join("media/audio")).unwrap();
        assert!(session.load_audio(&id).is_err());
        assert!(outside.exists());
    }
}
