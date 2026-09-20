import { Channel, invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";
import {
  latexEnvironmentSchema, latexInstallProgressSchema, type LatexInstallProgress,
  deletedEntrySchema,
  entrySummarySchema,
  exportPreviewSchema,
  exportResultSchema,
  exportSettingsSchema,
  entrySortSettingsSchema, manualSortLayoutSchema,
  lexicalEntrySchema,
  projectSnapshotSchema,
  texEngineStatusSchema, fontPackStatusSchema, fontInstallProgressSchema,
  senseImageSchema, senseImageContentSchema, senseImageMutationSchema,
  audioAttachmentSchema, audioMutationSchema, audioContentSchema, type AudioOwner,
  csvInspectionSchema, csvImportPreviewSchema, csvImportResultSchema,
  type ExportKind,
  type ExportSettings,
  type EntrySortSettings, type ManualSortLayout,
  type LexicalEntry,
  type FontInstallProgress, type ProjectSnapshot,
  type CsvDelimiter, type CsvPreviewRequest,
  type WritingSystem,
} from "../types/domain";

const commandErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.string().optional(),
});

export class CommandError extends Error {
  readonly code: string;
  readonly details?: string;

  constructor(code: string, message: string, details?: string) {
    super(message);
    this.name = "CommandError";
    this.code = code;
    this.details = details;
  }
}

async function call<T>(
  command: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>,
): Promise<T> {
  try {
    return schema.parse(await invoke(command, args));
  } catch (error) {
    const parsed = commandErrorSchema.safeParse(error);
    if (parsed.success) {
      throw new CommandError(
        parsed.data.code,
        parsed.data.message,
        parsed.data.details,
      );
    }
    throw error;
  }
}

export const exportHelpUrls = {
  "upload": "https://docs.overleaf.com/managing-projects-and-files/uploading-a-project",
  "compiler": "https://docs.overleaf.com/getting-started/recompiling-your-project/selecting-a-tex-live-version-and-latex-compiler",
  "main": "https://docs.overleaf.com/getting-started/recompiling-your-project/the-main-document",
  "compile": "https://docs.overleaf.com/getting-started/recompiling-your-project",
  "download": "https://docs.overleaf.com/managing-projects-and-files/downloading-a-project",
  "texlive": "https://tug.org/texlive/tlmgr.html",
  "mactex": "https://tug.org/mactex/",
  "miktex": "https://miktex.org/howto/miktex-console"
} as const;

export const backend = {
  openLanguageCodeRegistry(): Promise<void> {
    return openUrl("https://iso639-3.sil.org/code_tables/639/data");
  },

  openScriptCodeRegistry(): Promise<void> {
    return openUrl("https://www.unicode.org/iso15924/iso15924-codes.html");
  },

  openOverleaf(): Promise<void> {
    return openUrl("https://www.overleaf.com/project");
  },

  openOverleafCompilerHelp(): Promise<void> {
    return openUrl("https://www.overleaf.com/learn/how-to/Changing_compiler");
  },

  openExportHelp(topic: "upload" | "compiler" | "main" | "compile" | "download" | "texlive" | "mactex" | "miktex"): Promise<void> {
    return openUrl(exportHelpUrls[topic]);
  },
  checkLatexEnvironment() {
    return call("check_latex_environment", {}, latexEnvironmentSchema);
  },
  installLatex(onProgress: (progress: LatexInstallProgress) => void): Promise<void> {
    const channel = new Channel<unknown>();
    channel.onmessage = (value) => onProgress(latexInstallProgressSchema.parse(value));
    return call("install_latex", { onProgress: channel }, z.null()).then(() => undefined);
  },
  cancelLatexDownload(): Promise<void> {
    return call("cancel_latex_download", {}, z.null()).then(() => undefined);
  },
  saveLatexInstallGuide(destination: string): Promise<string> {
    return call("save_latex_install_guide", { destination }, z.string());
  },

  async chooseFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },

  async chooseCsvFile(): Promise<string | null> {
    const selected = await open({ directory: false, multiple: false, filters: [{ name: "CSV", extensions: ["csv", "tsv", "txt"] }] });
    return typeof selected === "string" ? selected : null;
  },

  inspectCsv(path: string, delimiter?: CsvDelimiter) {
    return call("inspect_csv", { path, delimiter: delimiter ?? null }, csvInspectionSchema);
  },

  previewCsvImport(request: CsvPreviewRequest) {
    return call("preview_csv_import", { request }, csvImportPreviewSchema);
  },

  createProjectFromCsv(preview: CsvPreviewRequest, previewToken: string) {
    return call("create_project_from_csv", { request: { preview, previewToken } }, csvImportResultSchema);
  },

  async chooseCsvDestination(defaultPath: string): Promise<string | null> {
    const selected = await save({
      defaultPath,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    return typeof selected === "string" ? selected : null;
  },

  createProject(request: {
    parentDir: string;
    name: string;
    languageName: string | null;
    languageCode: string | null;
  }): Promise<ProjectSnapshot> {
    return call("create_project", { request }, projectSnapshotSchema);
  },

  openProject(path: string): Promise<ProjectSnapshot> {
    return call("open_project", { path }, projectSnapshotSchema);
  },

  closeProject(): Promise<void> {
    return call("close_project", {}, z.null()).then(() => undefined);
  },

  updateProjectSettings(request: {
    name: string;
    languageName: string | null;
    languageCode: string | null;
    analysisLanguage: "zh-TW" | "en" | null;
    description: string | null;
    writingSystems: WritingSystem[];
    partOfSpeechOptions: string[];
    semanticDomainOptions: string[];
  }): Promise<ProjectSnapshot> {
    return call("update_project_settings", { request }, projectSnapshotSchema);
  },

  saveExportSettings(settings: ExportSettings): Promise<ExportSettings> {
    return call("save_export_settings", { settings }, exportSettingsSchema);
  },

  saveEntrySortSettings(settings: EntrySortSettings): Promise<ProjectSnapshot> {
    entrySortSettingsSchema.parse(settings);
    return call("save_entry_sort_settings", { settings }, projectSnapshotSchema);
  },

  saveManualSortLayout(layout: ManualSortLayout): Promise<ProjectSnapshot> {
    manualSortLayoutSchema.parse(layout);
    return call("save_manual_sort_layout", { layout }, projectSnapshotSchema);
  },

  previewExport(kind: ExportKind) {
    return call("preview_export", { kind }, exportPreviewSchema);
  },

  exportProject(request: {
    kind: ExportKind;
    destination: string;
    snapshotToken: string;
    overwrite: boolean;
  }) {
    return call("export_project", { request }, exportResultSchema);
  },

  detectXeLatex() {
    return call("detect_xelatex", {}, texEngineStatusSchema);
  },

  listFontPacks() {
    return call("list_font_packs", {}, z.array(fontPackStatusSchema));
  },

  installFontPack(packId: string) {
    return call("install_font_pack", { packId }, fontPackStatusSchema);
  },

  installFontPacks(packIds: string[], onProgress: (progress: FontInstallProgress) => void) {
    const channel = new Channel<unknown>();
    channel.onmessage = (message) => onProgress(fontInstallProgressSchema.parse(message));
    return call("install_font_packs", { packIds, onProgress: channel }, z.array(fontPackStatusSchema));
  },

  queryEntries(query: string) {
    return call(
      "query_entry_summaries",
      { query },
      z.array(entrySummarySchema),
    );
  },

  loadEntry(id: string): Promise<LexicalEntry> {
    return call("load_entry", { id }, lexicalEntrySchema);
  },

  createEntry(): Promise<LexicalEntry> {
    return call("create_entry", {}, lexicalEntrySchema);
  },

  saveEntry(entry: LexicalEntry): Promise<LexicalEntry> {
    return call(
      "save_entry",
      { request: { entry, expectedRevision: entry.revision } },
      lexicalEntrySchema,
    );
  },

  async chooseAudioFiles(): Promise<string[]> {
    const selected = await open({ multiple: true, directory: false, filters: [
      { name: "WAV / MP3 / M4A / AAC / FLAC / OGG / Opus / AIFF / WebM", extensions: ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "aif", "aiff", "aifc", "webm"] },
    ] });
    return selected ? (Array.isArray(selected) ? selected : [selected]) : [];
  },
  listAudio(owner: AudioOwner) {
    return call("list_audio", { owner }, z.array(audioAttachmentSchema));
  },
  importAudio(request: { entryId: string; owner: AudioOwner; expectedRevision: number; sourcePath: string }) {
    return call("import_audio", { request }, audioMutationSchema);
  },
  beginAudioRecording(request: { entryId: string; owner: AudioOwner; expectedRevision: number }) {
    return call("begin_audio_recording", { request: { ...request, sourcePath: "" } }, z.string());
  },
  saveAudioRecording(request: { entryId: string; owner: AudioOwner; expectedRevision: number; sessionToken: string; mimeType: string; originalFilename: string; dataBase64: string }) {
    return call("save_audio_recording", { request }, audioMutationSchema);
  },
  loadAudio(audioId: string) {
    return call("load_audio", { audioId }, audioContentSchema);
  },
  removeAudio(request: { entryId: string; audioId: string; expectedRevision: number }) {
    return call("remove_audio", { request }, audioMutationSchema);
  },

  listSenseImages(senseId: string) {
    return call("list_sense_images", { senseId }, z.array(senseImageSchema));
  },

  attachSenseImage(request: {
    entryId: string;
    senseId: string;
    expectedRevision: number;
    originalFilename: string;
    pngBase64: string;
  }) {
    return call("attach_sense_image", { request }, senseImageMutationSchema);
  },

  loadSenseImage(imageId: string) {
    return call("load_sense_image", { imageId }, senseImageContentSchema);
  },

  removeSenseImage(request: {
    entryId: string;
    imageId: string;
    expectedRevision: number;
  }) {
    return call("remove_sense_image", { request }, senseImageMutationSchema);
  },

  deleteEntry(id: string, expectedRevision: number) {
    return call(
      "delete_entry",
      { request: { id, expectedRevision } },
      deletedEntrySchema,
    );
  },

  restoreEntry(id: string): Promise<LexicalEntry> {
    return call("restore_entry", { id }, lexicalEntrySchema);
  },
};
