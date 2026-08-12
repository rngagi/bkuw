import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";
import {
  deletedEntrySchema,
  entrySummarySchema,
  lexicalEntrySchema,
  projectSnapshotSchema,
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

  async chooseFolder(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
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
    description: string | null;
    writingSystems: WritingSystem[];
    partOfSpeechOptions: string[];
    semanticDomainOptions: string[];
  }): Promise<ProjectSnapshot> {
    return call("update_project_settings", { request }, projectSnapshotSchema);
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
