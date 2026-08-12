use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use chrono::Utc;
use csv::{Terminator, WriterBuilder};
use icu_collator::{Collator, options::CollatorOptions};
use icu_locale::Locale;
use serde::Serialize;
use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use wait_timeout::ChildExt;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    domain::{
        CorpusPartOfSpeech, ExportIssue, ExportIssueSeverity, ExportKind, ExportPreview,
        ExportProjectRequest, ExportResult, ExportSettingsV1, LexicalEntry, OmittedExportData,
        PdfStatus, Project, TexEngineStatus, WritingSystem,
    },
    error::{AppError, AppResult},
    font_manager::{
        CHARIS_PACK_ID, FontManager, NOTO_CJK_TC_PACK_ID, NOTO_SERIF_PACK_ID, NOTO_THAI_PACK_ID,
        NOTO_TIBETAN_PACK_ID, TERMES_PACK_ID,
    },
};

pub(crate) const CORPUS_HEADERS: [&str; 9] = [
    "form",
    "gloss_zh",
    "word_root",
    "example",
    "example_translation_zh",
    "ipa",
    "part_of_speech",
    "gloss_en",
    "notes",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportSnapshot {
    pub project: Project,
    pub writing_systems: Vec<WritingSystem>,
    pub settings: ExportSettingsV1,
    pub entries: Vec<LexicalEntry>,
}

#[derive(Debug, Clone)]
struct CorpusRow {
    form: String,
    gloss_zh: String,
    word_root: String,
    example: String,
    example_translation_zh: String,
    ipa: String,
    part_of_speech: String,
    gloss_en: String,
    notes: String,
    entry_id: String,
    sense_order: i64,
}

pub(crate) fn preview(
    snapshot: &ExportSnapshot,
    kind: ExportKind,
    fonts: Option<&FontManager>,
) -> AppResult<ExportPreview> {
    let token = snapshot_token(snapshot, kind)?;
    match kind {
        ExportKind::CorpusCsv => {
            let (rows, issues, omitted) = corpus_rows(snapshot);
            Ok(ExportPreview {
                snapshot_token: token,
                row_count: rows.len(),
                issues,
                omitted,
                required_font_packs: Vec::new(),
            })
        }
        ExportKind::Latex | ExportKind::Pdf => {
            let required_ids = required_font_pack_ids(snapshot);
            let required_font_packs = fonts
                .map(|manager| {
                    required_ids
                        .iter()
                        .filter_map(|id| manager.status_for(id).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let issues = latex_issues(snapshot, &required_font_packs);
            Ok(ExportPreview {
                snapshot_token: token,
                row_count: snapshot.entries.len(),
                issues,
                omitted: OmittedExportData::default(),
                required_font_packs,
            })
        }
    }
}

pub(crate) fn run(
    snapshot: &ExportSnapshot,
    request: ExportProjectRequest,
    fonts: Option<&FontManager>,
) -> AppResult<ExportResult> {
    let current = preview(snapshot, request.kind, fonts)?;
    if current.snapshot_token != request.snapshot_token {
        return Err(AppError::new(
            "export_stale",
            "The project changed after the export preview.",
        ));
    }
    if current.has_errors() {
        return Err(AppError::new(
            "export_validation",
            "Resolve the export validation errors before exporting.",
        ));
    }
    match request.kind {
        ExportKind::CorpusCsv => {
            let (rows, issues, _) = corpus_rows(snapshot);
            let bytes = encode_corpus_csv(&rows)?;
            let path = PathBuf::from(&request.destination);
            atomic_write(&path, &bytes, request.overwrite)?;
            Ok(ExportResult {
                csv_path: Some(path.to_string_lossy().into_owned()),
                latex_directory: None,
                zip_path: None,
                pdf_path: None,
                pdf_status: PdfStatus::NotRequested,
                row_count: rows.len(),
                issues,
                diagnostic_path: None,
            })
        }
        ExportKind::Latex | ExportKind::Pdf => {
            let manager = fonts.ok_or_else(|| {
                AppError::new(
                    "export_validation",
                    "Portable fonts were not validated for this export.",
                )
            })?;
            export_latex_project(snapshot, request.kind, &request.destination, manager)
        }
    }
}

#[must_use]
pub(crate) fn detect_xelatex() -> TexEngineStatus {
    let path = xelatex_configuration().0;
    TexEngineStatus {
        available: path.is_some(),
        path: path.map(|value| value.to_string_lossy().into_owned()),
    }
}

fn latex_issues(
    snapshot: &ExportSnapshot,
    required_font_packs: &[crate::domain::FontPackStatus],
) -> Vec<ExportIssue> {
    let mut issues = Vec::new();
    for pack in required_font_packs {
        let code = match pack.state {
            crate::domain::FontPackState::Installed => continue,
            crate::domain::FontPackState::Missing => "latex.font_pack_missing",
            crate::domain::FontPackState::Invalid => "latex.font_pack_invalid",
        };
        issues.push(issue(
            ExportIssueSeverity::Error,
            code,
            None,
            None,
            Some("fontPacks"),
            Some(&pack.id),
        ));
    }
    let headword_id = snapshot.settings.latex.headword_writing_system_id.as_str();
    if !snapshot
        .writing_systems
        .iter()
        .any(|system| system.id == headword_id)
    {
        issues.push(issue(
            ExportIssueSeverity::Error,
            "latex.headword_writing_system_required",
            None,
            None,
            Some("headwordWritingSystemId"),
            None,
        ));
    }
    for entry in &snapshot.entries {
        if form_text(entry, headword_id).is_none_or(|text| text.trim().is_empty()) {
            issues.push(issue(
                ExportIssueSeverity::Error,
                "latex.headword_required",
                Some(&entry.id),
                None,
                Some("forms"),
                None,
            ));
        }
    }
    if snapshot.entries.is_empty() {
        issues.push(issue(
            ExportIssueSeverity::Error,
            "latex.no_entries",
            None,
            None,
            None,
            None,
        ));
    }
    issues
}

fn export_latex_project(
    snapshot: &ExportSnapshot,
    kind: ExportKind,
    destination: &str,
    fonts: &FontManager,
) -> AppResult<ExportResult> {
    let parent = PathBuf::from(destination);
    if !parent.is_dir() {
        return Err(AppError::new(
            "export_filesystem",
            "Choose an existing folder for the LaTeX export.",
        ));
    }
    let base_name = export_base_name(&snapshot.project.name);
    let project_dir = unique_export_path(&parent, &format!("{base_name}-latex"), "");
    fs::create_dir(&project_dir).map_err(|error| export_io(error, "create LaTeX project"))?;
    let sources = render_latex_sources(snapshot, fonts)?;
    let source_result: AppResult<PathBuf> = (|| {
        for (name, contents) in &sources {
            let path = project_dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| export_io(error, &format!("create directory for {name}")))?;
            }
            fs::write(path, contents)
                .map_err(|error| export_io(error, &format!("write {name}")))?;
        }
        let zip_path = parent.join(format!(
            "{}-overleaf.zip",
            project_dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("bkuw-latex")
        ));
        let zip_bytes = encode_source_zip(&sources)?;
        atomic_write(&zip_path, &zip_bytes, false)?;
        Ok(zip_path)
    })();
    if source_result.is_err() {
        let _ = fs::remove_dir_all(&project_dir);
    }
    let zip_path = source_result?;
    let (pdf_path, pdf_status) = if kind == ExportKind::Pdf {
        match compile_xelatex(&project_dir, &sources)? {
            Some(path) => (
                Some(path.to_string_lossy().into_owned()),
                PdfStatus::Created,
            ),
            None => (None, PdfStatus::XeLatexMissing),
        }
    } else {
        (None, PdfStatus::NotRequested)
    };
    Ok(ExportResult {
        csv_path: None,
        latex_directory: Some(project_dir.to_string_lossy().into_owned()),
        zip_path: Some(zip_path.to_string_lossy().into_owned()),
        pdf_path,
        pdf_status,
        row_count: snapshot.entries.len(),
        issues: Vec::new(),
        diagnostic_path: None,
    })
}

fn compile_xelatex(
    project_dir: &Path,
    sources: &[(String, Vec<u8>)],
) -> AppResult<Option<PathBuf>> {
    let (engine, timeout) = xelatex_configuration();
    let Some(engine) = engine else {
        return Ok(None);
    };
    let build = tempfile::Builder::new()
        .prefix("bkuw-xelatex-")
        .tempdir()
        .map_err(|error| export_io(error, "create isolated XeLaTeX build directory"))?;
    for (name, contents) in sources {
        let path = build.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                export_io(error, &format!("create staging directory for {name}"))
            })?;
        }
        fs::write(path, contents)
            .map_err(|error| export_io(error, &format!("stage {name} for XeLaTeX")))?;
    }
    let diagnostic_path = project_dir.join("diagnostic.log");
    let mut diagnostic = fs::File::create(&diagnostic_path)
        .map_err(|error| export_io(error, "create XeLaTeX diagnostic log"))?;
    for pass in 1..=2 {
        writeln!(diagnostic, "bkuw XeLaTeX pass {pass}")
            .map_err(|error| export_io(error, "write XeLaTeX diagnostic log"))?;
        diagnostic
            .flush()
            .map_err(|error| export_io(error, "flush XeLaTeX diagnostic log"))?;
        let stdout = diagnostic
            .try_clone()
            .map_err(|error| export_io(error, "open XeLaTeX stdout log"))?;
        let stderr = diagnostic
            .try_clone()
            .map_err(|error| export_io(error, "open XeLaTeX stderr log"))?;
        let mut child = Command::new(&engine)
            .args([
                "-no-shell-escape",
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-file-line-error",
                "main.tex",
            ])
            .current_dir(build.path())
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| {
                let _ = writeln!(diagnostic, "failed to start XeLaTeX: {error}");
                AppError::with_details(
                    "latex_compile",
                    "XeLaTeX could not be started. The source project was preserved.",
                    diagnostic_path.to_string_lossy(),
                )
            })?;
        let status = child.wait_timeout(timeout).map_err(|error| {
            AppError::with_details(
                "latex_compile",
                format!("Waiting for XeLaTeX failed: {error}"),
                diagnostic_path.to_string_lossy(),
            )
        })?;
        let Some(status) = status else {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writeln!(
                diagnostic,
                "XeLaTeX exceeded the {} second timeout.",
                timeout.as_secs_f64()
            );
            return Err(AppError::with_details(
                "latex_timeout",
                "XeLaTeX timed out. The source project and diagnostic log were preserved.",
                diagnostic_path.to_string_lossy(),
            ));
        };
        if !status.success() {
            let _ = writeln!(diagnostic, "XeLaTeX exited with status {status}.");
            return Err(AppError::with_details(
                "latex_compile",
                "XeLaTeX could not compile the PDF. The source project and diagnostic log were preserved.",
                diagnostic_path.to_string_lossy(),
            ));
        }
    }
    let built_pdf = build.path().join("main.pdf");
    if !built_pdf.is_file() {
        let _ = writeln!(diagnostic, "XeLaTeX completed without creating main.pdf.");
        return Err(AppError::with_details(
            "latex_compile",
            "XeLaTeX did not create a PDF. The source project and diagnostic log were preserved.",
            diagnostic_path.to_string_lossy(),
        ));
    }
    let pdf_path = project_dir.join("dictionary.pdf");
    fs::copy(&built_pdf, &pdf_path)
        .map_err(|error| export_io(error, "copy compiled PDF into the export project"))?;
    drop(diagnostic);
    let _ = fs::remove_file(&diagnostic_path);
    Ok(Some(pdf_path))
}

fn xelatex_configuration() -> (Option<PathBuf>, Duration) {
    #[cfg(all(test, unix))]
    if let Some(configuration) = TEST_XELATEX_OVERRIDE
        .lock()
        .expect("test XeLaTeX override")
        .clone()
    {
        return (configuration.path, configuration.timeout);
    }
    (find_xelatex(), Duration::from_secs(120))
}

fn find_xelatex() -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        "xelatex.exe"
    } else {
        "xelatex"
    };
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let mut candidates = vec![PathBuf::from("/Library/TeX/texbin/xelatex")];
    if cfg!(windows) {
        candidates.extend([
            PathBuf::from(r"C:\Program Files\MiKTeX\miktex\bin\x64\xelatex.exe"),
            PathBuf::from(r"C:\Program Files\MiKTeX\miktex\bin\xelatex.exe"),
        ]);
        for year in 2020..=2030 {
            candidates.push(PathBuf::from(format!(
                r"C:\texlive\{year}\bin\windows\xelatex.exe"
            )));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

#[cfg(all(test, unix))]
#[derive(Clone)]
struct TestXeLatexConfiguration {
    path: Option<PathBuf>,
    timeout: Duration,
}

#[cfg(all(test, unix))]
static TEST_XELATEX_OVERRIDE: std::sync::Mutex<Option<TestXeLatexConfiguration>> =
    std::sync::Mutex::new(None);

#[cfg(all(test, unix))]
static TEST_XELATEX_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, unix))]
pub(crate) fn with_test_xelatex<R>(
    path: Option<PathBuf>,
    timeout: Duration,
    operation: impl FnOnce() -> R,
) -> R {
    let _serial = TEST_XELATEX_SERIAL.lock().expect("serialize XeLaTeX tests");
    *TEST_XELATEX_OVERRIDE.lock().expect("set XeLaTeX override") =
        Some(TestXeLatexConfiguration { path, timeout });
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            *TEST_XELATEX_OVERRIDE
                .lock()
                .expect("reset XeLaTeX override") = None;
        }
    }
    let _reset = Reset;
    operation()
}

fn render_latex_sources(
    snapshot: &ExportSnapshot,
    fonts: &FontManager,
) -> AppResult<Vec<(String, Vec<u8>)>> {
    let required_ids = required_font_pack_ids(snapshot);
    let font_definitions = font_definitions(snapshot, fonts)?;
    let mut main = include_str!("../templates/latex/main.tex").to_owned();
    main = main.replace("{{FONT_DEFINITIONS}}", &font_definitions);
    main = main.replace("{{TITLE}}", &tex_escape(&snapshot.settings.latex.title));
    main = main.replace("{{AUTHOR}}", &tex_escape(&snapshot.settings.latex.author));
    main = main.replace(
        "{{REVERSE_INDEX}}",
        if snapshot.settings.latex.reverse_index == crate::domain::ReverseIndexMode::Gloss {
            "\\clearpage\n\\section*{{\\bkuwanalysisfont Reverse index / 逆向索引}}\n\\input{reverse-index.tex}"
        } else {
            ""
        },
    );
    let entries = render_entries(snapshot)?;
    let reverse = render_reverse_index(snapshot)?;
    let mut sources = vec![
        ("main.tex".into(), main.into_bytes()),
        ("entries.tex".into(), entries.into_bytes()),
        ("reverse-index.tex".into(), reverse.into_bytes()),
        (
            ".latexmkrc".into(),
            b"$pdf_mode = 5;\n$xelatex = 'xelatex -no-shell-escape -interaction=nonstopmode -halt-on-error -file-line-error %O %S';\n".to_vec(),
        ),
        (
            "README.md".into(),
            include_str!("../templates/latex/README.md").as_bytes().to_vec(),
        ),
    ];
    sources.extend(fonts.export_files(&required_ids)?);
    Ok(sources)
}

fn render_entries(snapshot: &ExportSnapshot) -> AppResult<String> {
    let headword_id = snapshot.settings.latex.headword_writing_system_id.as_str();
    let pronunciation_id = snapshot
        .settings
        .latex
        .pronunciation_writing_system_id
        .as_deref();
    let mut entries = snapshot.entries.iter().collect::<Vec<_>>();
    sort_entries(
        &mut entries,
        headword_id,
        snapshot.settings.latex.collation_language_tag.as_deref(),
    );
    let systems = snapshot
        .writing_systems
        .iter()
        .map(|system| (system.id.as_str(), system))
        .collect::<BTreeMap<_, _>>();
    let section_mode = &snapshot.settings.latex.section_mode;
    let script = systems
        .get(headword_id)
        .and_then(|system| system.script_code.as_deref());
    let mut current_section: Option<String> = None;
    let mut output = String::new();
    for entry in entries {
        let headword = form_text(entry, headword_id).unwrap_or_default();
        let section = section_for(headword, section_mode, script);
        if section.is_some() && section != current_section {
            current_section.clone_from(&section);
            output.push_str(&format!(
                "\\BkuwSection{{{}}}\n",
                tex_escape(section.as_deref().unwrap_or_default())
            ));
        }
        let headword_tex = ws_text(snapshot, headword_id, headword);
        let pronunciation_tex = pronunciation_id
            .and_then(|id| form_text(entry, id).map(|text| ws_display_text(snapshot, id, text)))
            .unwrap_or_default();
        output.push_str(&format!(
            "\\BkuwEntry{{entry:{}}}{{{}}}{{{}}}\n",
            entry.id, headword_tex, pronunciation_tex
        ));
        let other_forms = entry
            .forms
            .iter()
            .filter(|form| !form.text.trim().is_empty() && form.writing_system_id != headword_id)
            .map(|form| {
                let name = systems
                    .get(form.writing_system_id.as_str())
                    .map(|system| tex_escape(&system.name))
                    .unwrap_or_default();
                format!(
                    "{}: {}",
                    name,
                    ws_display_text(snapshot, &form.writing_system_id, &form.text)
                )
            })
            .collect::<Vec<_>>();
        if !other_forms.is_empty() {
            output.push_str(&format!("\\BkuwMeta{{{}}}\n", other_forms.join("; ")));
        }
        for (index, sense) in entry.senses.iter().enumerate() {
            output.push_str(&format!(
                "\\BkuwSense{{{}}}{{{}}}{{{}}}{{{}}}\n",
                index + 1,
                tex_escape(sense.part_of_speech.as_deref().unwrap_or_default()),
                tex_escape(sense.gloss.as_deref().unwrap_or_default()),
                tex_escape(sense.definition.as_deref().unwrap_or_default()),
            ));
            if let Some(domain) = &sense.semantic_domain {
                output.push_str(&format!(
                    "\\BkuwMeta{{Semantic domain: {}}}\n",
                    tex_escape(domain)
                ));
            }
            for example in &sense.examples {
                let mut forms = example.forms.iter().collect::<Vec<_>>();
                forms.sort_by_key(|form| {
                    (
                        form.writing_system_id != snapshot.settings.latex.example_writing_system_id,
                        form.sort_order,
                    )
                });
                let rendered = forms
                    .into_iter()
                    .filter(|form| !form.text.trim().is_empty())
                    .map(|form| ws_display_text(snapshot, &form.writing_system_id, &form.text))
                    .collect::<Vec<_>>()
                    .join(" / ");
                let translation = tex_escape(example.translation.as_deref().unwrap_or_default());
                let notes = tex_escape(example.notes.as_deref().unwrap_or_default());
                let mut parts = vec![rendered];
                if !translation.is_empty() {
                    parts.push(format!("({translation})"));
                }
                if !notes.is_empty() {
                    parts.push(format!("— {notes}"));
                }
                output.push_str(&format!("\\BkuwExample{{{}}}\n", parts.join(" ")));
            }
        }
        let relation_text = entry
            .relations
            .iter()
            .filter_map(|relation| {
                relation.fallback_text.as_deref().map(|label| {
                    format!(
                        "{}: {}",
                        tex_escape(&relation.relation_type),
                        tex_escape(label)
                    )
                })
            })
            .collect::<Vec<_>>();
        if !relation_text.is_empty() {
            output.push_str(&format!("\\BkuwMeta{{{}}}\n", relation_text.join("; ")));
        }
        if let Some(notes) = &entry.notes {
            output.push_str(&format!("\\BkuwMeta{{Notes: {}}}\n", tex_escape(notes)));
        }
    }
    Ok(output)
}

fn render_reverse_index(snapshot: &ExportSnapshot) -> AppResult<String> {
    if snapshot.settings.latex.reverse_index == crate::domain::ReverseIndexMode::None {
        return Ok("% Reverse index disabled.\n".into());
    }
    let headword_id = snapshot.settings.latex.headword_writing_system_id.as_str();
    let mut rows = snapshot
        .entries
        .iter()
        .flat_map(|entry| {
            let headword = form_text(entry, headword_id).unwrap_or_default();
            entry.senses.iter().filter_map(move |sense| {
                sense
                    .gloss
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|gloss| (gloss, headword, entry.id.as_str(), sense.sort_order))
            })
        })
        .collect::<Vec<_>>();
    let locale = snapshot
        .project
        .analysis_language
        .as_deref()
        .unwrap_or("und");
    let collator = collator(locale);
    rows.sort_by(|left, right| {
        compare_with(collator.as_ref(), left.0, right.0)
            .then_with(|| left.2.cmp(right.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    Ok(rows
        .into_iter()
        .map(|(gloss, headword, id, _)| {
            format!(
            "\\noindent {{\\bkuwanalysisfont {}}} \\dotfill \\hyperlink{{entry:{}}}{{{}}} (p.~\\pageref{{entry:{}}})\\par\n",
                tex_escape(gloss), id, ws_text(snapshot, headword_id, headword), id
            )
        })
        .collect())
}

fn sort_entries(entries: &mut [&LexicalEntry], headword_id: &str, language_tag: Option<&str>) {
    let collator = collator(language_tag.unwrap_or("und"));
    entries.sort_by(|left, right| {
        compare_with(
            collator.as_ref(),
            form_text(left, headword_id).unwrap_or_default(),
            form_text(right, headword_id).unwrap_or_default(),
        )
        .then_with(|| left.id.cmp(&right.id))
    });
}

fn collator(language_tag: &str) -> Option<icu_collator::CollatorBorrowed<'static>> {
    let locale = language_tag.parse::<Locale>().ok()?;
    Collator::try_new(locale.into(), CollatorOptions::default()).ok()
}

fn compare_with(
    collator: Option<&icu_collator::CollatorBorrowed<'_>>,
    left: &str,
    right: &str,
) -> std::cmp::Ordering {
    collator
        .map(|value| value.compare(left, right))
        .unwrap_or_else(|| left.cmp(right))
}

fn section_for(
    headword: &str,
    mode: &crate::domain::SectionMode,
    script: Option<&str>,
) -> Option<String> {
    let enabled = match mode {
        crate::domain::SectionMode::None => false,
        crate::domain::SectionMode::FirstGrapheme => true,
        crate::domain::SectionMode::Auto => matches!(script, Some("Latn" | "Cyrl" | "Grek")),
    };
    enabled.then(|| {
        headword
            .graphemes(true)
            .next()
            .unwrap_or("#")
            .to_uppercase()
    })
}

fn ws_text(snapshot: &ExportSnapshot, writing_system_id: &str, text: &str) -> String {
    let index = snapshot
        .writing_systems
        .iter()
        .position(|system| system.id == writing_system_id)
        .unwrap_or_default();
    format!("{{\\{} {}}}", font_command_name(index), tex_escape(text))
}

fn ws_display_text(snapshot: &ExportSnapshot, writing_system_id: &str, text: &str) -> String {
    let rendered = ws_text(snapshot, writing_system_id, text);
    match snapshot
        .writing_systems
        .iter()
        .find(|system| system.id == writing_system_id)
        .map(|system| system.kind.as_str())
    {
        Some("phonemic") => format!("/{rendered}/"),
        Some("phonetic") => format!("[{rendered}]"),
        _ => rendered,
    }
}

fn font_definitions(snapshot: &ExportSnapshot, fonts: &FontManager) -> AppResult<String> {
    let mut definitions = vec![font_command("setmainfont", TERMES_PACK_ID, fonts)?];
    let analysis_pack = if snapshot.project.analysis_language.as_deref() == Some("zh-TW") {
        NOTO_CJK_TC_PACK_ID
    } else {
        TERMES_PACK_ID
    };
    definitions.push(font_command(
        "newfontfamily\\bkuwanalysisfont",
        analysis_pack,
        fonts,
    )?);
    definitions.extend(
        snapshot
            .writing_systems
            .iter()
            .enumerate()
            .map(|(index, system)| {
                let preset = snapshot
                    .settings
                    .latex
                    .font_presets
                    .get(&system.id)
                    .unwrap_or(&crate::domain::FontPreset::Auto);
                let pack_id =
                    portable_font_pack_id(preset, system.script_code.as_deref(), &system.kind);
                let command = font_command_name(index);
                font_command(&format!("newfontfamily\\{command}"), pack_id, fonts)
            })
            .collect::<AppResult<Vec<_>>>()?,
    );
    Ok(definitions.join("\n"))
}

fn font_command(command: &str, pack_id: &str, fonts: &FontManager) -> AppResult<String> {
    let family = fonts.family(pack_id)?;
    let mut options = vec![format!("Path=fonts/{pack_id}/")];
    if let Some(file) = family.bold {
        options.push(format!("BoldFont={file}"));
    }
    if let Some(file) = family.italic {
        options.push(format!("ItalicFont={file}"));
    }
    if let Some(file) = family.bold_italic {
        options.push(format!("BoldItalicFont={file}"));
    }
    Ok(format!(
        "\\{command}[{}]{{{}}}",
        options.join(","),
        family.regular
    ))
}

fn font_command_name(mut index: usize) -> String {
    let mut suffix = String::new();
    loop {
        suffix.insert(0, char::from(b'a' + (index % 26) as u8));
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    format!("bkuwfont{suffix}")
}

fn portable_font_pack_id(
    preset: &crate::domain::FontPreset,
    script: Option<&str>,
    writing_system_kind: &str,
) -> &'static str {
    if matches!(writing_system_kind, "phonemic" | "phonetic") {
        return CHARIS_PACK_ID;
    }
    match preset {
        crate::domain::FontPreset::CharisSil => CHARIS_PACK_ID,
        crate::domain::FontPreset::NotoSerif => NOTO_SERIF_PACK_ID,
        crate::domain::FontPreset::NotoSerifCjkTc => NOTO_CJK_TC_PACK_ID,
        crate::domain::FontPreset::NotoSerifTibetan => NOTO_TIBETAN_PACK_ID,
        crate::domain::FontPreset::NotoSerifThai => NOTO_THAI_PACK_ID,
        crate::domain::FontPreset::Auto => match script {
            Some("Hant") => NOTO_CJK_TC_PACK_ID,
            Some("Tibt") => NOTO_TIBETAN_PACK_ID,
            Some("Thai") => NOTO_THAI_PACK_ID,
            Some("Latn") => CHARIS_PACK_ID,
            _ => NOTO_SERIF_PACK_ID,
        },
    }
}

fn required_font_pack_ids(snapshot: &ExportSnapshot) -> Vec<String> {
    let mut ids = BTreeSet::from([TERMES_PACK_ID.to_owned()]);
    if snapshot.project.analysis_language.as_deref() == Some("zh-TW") {
        ids.insert(NOTO_CJK_TC_PACK_ID.to_owned());
    }
    for system in &snapshot.writing_systems {
        let preset = snapshot
            .settings
            .latex
            .font_presets
            .get(&system.id)
            .unwrap_or(&crate::domain::FontPreset::Auto);
        ids.insert(
            portable_font_pack_id(preset, system.script_code.as_deref(), &system.kind).to_owned(),
        );
    }
    ids.into_iter().collect()
}

fn tex_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '#' => escaped.push_str("\\#"),
            '$' => escaped.push_str("\\$"),
            '%' => escaped.push_str("\\%"),
            '&' => escaped.push_str("\\&"),
            '_' => escaped.push_str("\\_"),
            '^' => escaped.push_str("\\textasciicircum{}"),
            '~' => escaped.push_str("\\textasciitilde{}"),
            '\n' => escaped.push_str("\\par "),
            '\r' => {}
            other => escaped.push(other),
        }
    }
    escaped
}

fn encode_source_zip(sources: &[(String, Vec<u8>)]) -> AppResult<Vec<u8>> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for (name, contents) in sources {
        writer.start_file(name, options).map_err(zip_error)?;
        writer
            .write_all(contents)
            .map_err(|error| export_io(error, "write ZIP member"))?;
    }
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(zip_error)
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::with_details(
        "export_filesystem",
        "The Overleaf ZIP could not be generated.",
        error.to_string(),
    )
}

fn export_base_name(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        "bkuw".into()
    } else {
        cleaned.into()
    }
}

fn unique_export_path(parent: &Path, prefix: &str, extension: &str) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let suffix = &Uuid::new_v4().to_string()[..8];
    parent.join(format!("{prefix}-{timestamp}-{suffix}{extension}"))
}

fn snapshot_token(snapshot: &ExportSnapshot, kind: ExportKind) -> AppResult<String> {
    let bytes = serde_json::to_vec(&(snapshot, kind)).map_err(|error| {
        AppError::with_details(
            "internal",
            "The export snapshot could not be encoded.",
            error.to_string(),
        )
    })?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn corpus_rows(snapshot: &ExportSnapshot) -> (Vec<CorpusRow>, Vec<ExportIssue>, OmittedExportData) {
    let mut issues = Vec::new();
    let mut omitted = OmittedExportData::default();
    if snapshot.project.analysis_language.as_deref() != Some("zh-TW") {
        issues.push(issue(
            ExportIssueSeverity::Error,
            "corpus.analysis_language_required",
            None,
            None,
            Some("analysisLanguage"),
            None,
        ));
    }
    let primary_id = snapshot
        .writing_systems
        .iter()
        .find(|system| system.display_role.as_deref() == Some("primary"))
        .map(|system| system.id.as_str())
        .unwrap_or_default();
    let pronunciation_id = snapshot
        .settings
        .latex
        .pronunciation_writing_system_id
        .as_deref();
    let example_id = snapshot.settings.latex.example_writing_system_id.as_str();
    let labels = snapshot
        .entries
        .iter()
        .filter_map(|entry| {
            form_text(entry, primary_id).map(|text| (entry.id.as_str(), text.to_owned()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut candidates = BTreeSet::new();

    for entry in &snapshot.entries {
        let form = form_text(entry, primary_id)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if form.is_empty() {
            issues.push(issue(
                ExportIssueSeverity::Error,
                "corpus.primary_form_required",
                Some(&entry.id),
                None,
                Some("form"),
                None,
            ));
        }
        if entry.senses.is_empty() {
            issues.push(issue(
                ExportIssueSeverity::Error,
                "corpus.sense_required",
                Some(&entry.id),
                None,
                Some("senses"),
                None,
            ));
        }
        let roots = entry
            .relations
            .iter()
            .filter(|relation| relation.relation_type == "root")
            .filter_map(|relation| {
                relation
                    .target_entry_id
                    .as_deref()
                    .and_then(|id| labels.get(id).cloned())
                    .or_else(|| relation.fallback_text.clone())
            })
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>();
        for root in &roots {
            if root.contains(';') {
                issues.push(issue(
                    ExportIssueSeverity::Error,
                    "corpus.root_separator_unsupported",
                    Some(&entry.id),
                    None,
                    Some("wordRoot"),
                    Some(root),
                ));
            }
        }
        let bases = entry
            .relations
            .iter()
            .filter(|relation| relation.relation_type == "base")
            .count();
        omitted.base_relations += bases;
        if bases > 0 {
            issues.push(issue(
                ExportIssueSeverity::Warning,
                "corpus.base_relations_omitted",
                Some(&entry.id),
                None,
                Some("relations"),
                None,
            ));
        }
        let ipa = pronunciation_id
            .and_then(|id| form_text(entry, id))
            .unwrap_or_default()
            .to_owned();

        for sense in &entry.senses {
            let gloss = sense.gloss.as_deref().unwrap_or_default().trim().to_owned();
            if gloss.is_empty() {
                issues.push(issue(
                    ExportIssueSeverity::Error,
                    "corpus.gloss_zh_required",
                    Some(&entry.id),
                    Some(&sense.id),
                    Some("gloss"),
                    None,
                ));
            }
            let selected = sense.examples.iter().find_map(|example| {
                let text = example
                    .forms
                    .iter()
                    .find(|form| form.writing_system_id == example_id)
                    .map(|form| form.text.trim())
                    .filter(|value| !value.is_empty());
                let translation = example
                    .translation
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                text.zip(translation)
                    .map(|(text, translation)| (example, text, translation))
            });
            if sense.examples.len() > usize::from(selected.is_some()) {
                omitted.examples += sense.examples.len() - usize::from(selected.is_some());
                issues.push(issue(
                    ExportIssueSeverity::Warning,
                    "corpus.examples_omitted",
                    Some(&entry.id),
                    Some(&sense.id),
                    Some("examples"),
                    None,
                ));
            }
            if let Some((example, _, _)) = selected {
                let extra = example.forms.len().saturating_sub(1);
                omitted.example_forms += extra;
                if extra > 0 {
                    issues.push(issue(
                        ExportIssueSeverity::Warning,
                        "corpus.example_forms_omitted",
                        Some(&entry.id),
                        Some(&sense.id),
                        Some("exampleForms"),
                        None,
                    ));
                }
            }
            let part_of_speech = sense
                .part_of_speech
                .as_ref()
                .and_then(|value| snapshot.settings.corpus.part_of_speech_mappings.get(value))
                .map(corpus_pos_code)
                .unwrap_or_default()
                .to_owned();
            if sense.part_of_speech.is_some() && part_of_speech.is_empty() {
                issues.push(issue(
                    ExportIssueSeverity::Warning,
                    "corpus.part_of_speech_unmapped",
                    Some(&entry.id),
                    Some(&sense.id),
                    Some("partOfSpeech"),
                    sense.part_of_speech.as_deref(),
                ));
            }
            let notes = [
                entry
                    .notes
                    .as_deref()
                    .map(|value| format!("entry_notes: {value}")),
                sense
                    .definition
                    .as_deref()
                    .map(|value| format!("sense_definition: {value}")),
                sense
                    .semantic_domain
                    .as_deref()
                    .map(|value| format!("semantic_domain: {value}")),
                selected.and_then(|(example, _, _)| {
                    example
                        .notes
                        .as_deref()
                        .map(|value| format!("example_notes: {value}"))
                }),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\\n");
            let (example, translation) = selected
                .map(|(_, text, translation)| (text.to_owned(), translation.to_owned()))
                .unwrap_or_default();
            if !candidates.insert((form.to_lowercase(), gloss.clone())) {
                issues.push(issue(
                    ExportIssueSeverity::Warning,
                    "corpus.duplicate_candidate",
                    Some(&entry.id),
                    Some(&sense.id),
                    Some("form"),
                    None,
                ));
            }
            rows.push(CorpusRow {
                form: form.clone(),
                gloss_zh: gloss,
                word_root: roots.join(";"),
                example,
                example_translation_zh: translation,
                ipa: ipa.clone(),
                part_of_speech,
                gloss_en: String::new(),
                notes,
                entry_id: entry.id.clone(),
                sense_order: sense.sort_order,
            });
        }
    }
    if rows.is_empty() {
        issues.push(issue(
            ExportIssueSeverity::Error,
            "corpus.no_rows",
            None,
            None,
            None,
            None,
        ));
    }
    sort_corpus_rows(
        &mut rows,
        snapshot.settings.latex.collation_language_tag.as_deref(),
    );
    (rows, issues, omitted)
}

fn sort_corpus_rows(rows: &mut [CorpusRow], language_tag: Option<&str>) {
    let locale = language_tag
        .and_then(|tag| tag.parse::<Locale>().ok())
        .unwrap_or_else(|| "und".parse().expect("und is a valid locale"));
    let collator = Collator::try_new(locale.into(), CollatorOptions::default()).ok();
    rows.sort_by(|left, right| {
        collator
            .as_ref()
            .map(|value| value.compare(&left.form, &right.form))
            .unwrap_or_else(|| left.form.cmp(&right.form))
            .then_with(|| left.entry_id.cmp(&right.entry_id))
            .then_with(|| left.sense_order.cmp(&right.sense_order))
    });
}

fn form_text<'a>(entry: &'a LexicalEntry, writing_system_id: &str) -> Option<&'a str> {
    entry
        .forms
        .iter()
        .find(|form| form.writing_system_id == writing_system_id)
        .map(|form| form.text.as_str())
}

fn corpus_pos_code(value: &CorpusPartOfSpeech) -> &'static str {
    match value {
        CorpusPartOfSpeech::Noun => "noun",
        CorpusPartOfSpeech::Verb => "verb",
        CorpusPartOfSpeech::Adjective => "adjective",
        CorpusPartOfSpeech::Adverb => "adverb",
        CorpusPartOfSpeech::Pronoun => "pronoun",
        CorpusPartOfSpeech::Particle => "particle",
        CorpusPartOfSpeech::Other => "other",
    }
}

fn issue(
    severity: ExportIssueSeverity,
    code: &str,
    entry_id: Option<&str>,
    sense_id: Option<&str>,
    field: Option<&str>,
    details: Option<&str>,
) -> ExportIssue {
    ExportIssue {
        severity,
        code: code.to_owned(),
        entry_id: entry_id.map(ToOwned::to_owned),
        sense_id: sense_id.map(ToOwned::to_owned),
        field: field.map(ToOwned::to_owned),
        details: details.map(ToOwned::to_owned),
    }
}

fn encode_corpus_csv(rows: &[CorpusRow]) -> AppResult<Vec<u8>> {
    let mut writer = WriterBuilder::new()
        .terminator(Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record(CORPUS_HEADERS).map_err(csv_error)?;
    for row in rows {
        writer
            .write_record([
                &row.form,
                &row.gloss_zh,
                &row.word_root,
                &row.example,
                &row.example_translation_zh,
                &row.ipa,
                &row.part_of_speech,
                &row.gloss_en,
                &row.notes,
            ])
            .map_err(csv_error)?;
    }
    writer
        .into_inner()
        .map_err(|error| csv_error(error.into_error().into()))
}

fn csv_error(error: csv::Error) -> AppError {
    AppError::with_details(
        "export_filesystem",
        "The corpus CSV could not be generated.",
        error.to_string(),
    )
}

fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new("export_filesystem", "Choose a destination inside a folder.")
    })?;
    if !parent.is_dir() {
        return Err(AppError::new(
            "export_filesystem",
            "The export destination folder does not exist.",
        ));
    }
    if path.exists() && !overwrite {
        return Err(AppError::with_details(
            "export_filesystem",
            "The export destination already exists.",
            "destination_exists",
        ));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("bkuw-export");
    let temporary = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, bytes).map_err(|error| export_io(error, "write temporary export"))?;
    commit_temporary(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        export_io(error, "commit export")
    })?;
    Ok(())
}

#[cfg(not(windows))]
fn commit_temporary(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn commit_temporary(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn export_io(error: std::io::Error, phase: &str) -> AppError {
    AppError::with_details(
        "export_filesystem",
        "The export file could not be written.",
        format!("{phase}: {error}"),
    )
}

#[cfg(test)]
mod font_selection_tests {
    use super::*;

    #[test]
    fn phonemic_and_phonetic_systems_always_use_charis_sil() {
        for kind in ["phonemic", "phonetic"] {
            assert_eq!(
                portable_font_pack_id(
                    &crate::domain::FontPreset::NotoSerifCjkTc,
                    Some("Latn"),
                    kind,
                ),
                CHARIS_PACK_ID,
            );
        }
    }
}
