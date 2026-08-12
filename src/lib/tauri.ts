import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";
import {
  deletedEntrySchema,
  entrySummarySchema,
  exportPreviewSchema,
  exportResultSchema,
  exportSettingsSchema,
  lexicalEntrySchema,
  projectSnapshotSchema,
  texEngineStatusSchema, fontPackStatusSchema,
  type ExportKind,
  type ExportSettings,
  type LexicalEntry,
  type ProjectSnapshot,
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

  async chooseFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
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
