import { z } from "zod";

const nullableText = z.string().nullable();

export const writingSystemSchema = z.object({
  id: z.string(),
  name: z.string(),
  type: z.enum([
    "orthography",
    "romanization",
    "transliteration",
    "phonemic",
    "phonetic",
    "other",
  ]),
  scriptCode: nullableText,
  languageTag: nullableText,
  displayRole: z.enum(["primary", "secondary"]).nullable(),
  sortOrder: z.number(),
  fontFamily: nullableText,
  notes: nullableText,
});

export const entryFormSchema = z.object({
  id: z.string(),
  writingSystemId: z.string(),
  text: z.string(),
  variantLabel: nullableText,
  dialect: nullableText,
  status: nullableText,
  notes: nullableText,
  sortOrder: z.number(),
});

export const exampleFormSchema = z.object({
  id: z.string(),
  writingSystemId: z.string(),
  text: z.string(),
  sortOrder: z.number(),
});

export const exampleSchema = z.object({
  id: z.string(),
  translation: nullableText,
  notes: nullableText,
  sortOrder: z.number(),
  forms: z.array(exampleFormSchema),
});

export const senseSchema = z.object({
  id: z.string(),
  gloss: nullableText,
  definition: nullableText,
  partOfSpeech: nullableText,
  semanticDomain: nullableText,
  sortOrder: z.number(),
  examples: z.array(exampleSchema),
});

export const relationSchema = z.object({
  id: z.string(),
  targetEntryId: nullableText,
  relationType: z.enum(["root", "base"]),
  fallbackText: nullableText,
  notes: nullableText,
  sortOrder: z.number(),
});

export const lexicalEntrySchema = z.object({
  id: z.string(),
  notes: nullableText,
  sectionOverride: nullableText,
  revision: z.number(),
  createdAt: z.string(),
  updatedAt: z.string(),
  forms: z.array(entryFormSchema),
  senses: z.array(senseSchema),
  relations: z.array(relationSchema),
});

export const entrySummarySchema = z.object({
  id: z.string(),
  primaryForm: z.string(),
  secondaryForm: nullableText,
  pronunciationForm: nullableText,
  pronunciationWritingSystemId: nullableText,
  senses: z.array(z.object({
    partOfSpeech: nullableText,
    gloss: nullableText,
  })),
  revision: z.number(),
  sectionLabel: nullableText,
  manualOrderPending: z.boolean(),
});

export const entrySortSettingsSchema = z.object({
  version: z.literal(1),
  mode: z.enum(["auto", "manual"]),
  writingSystemId: z.string(),
  alphabet: z.array(z.string()),
});

export const manualSortItemSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("heading"), id: z.string(), label: z.string() }),
  z.object({ kind: z.literal("entry"), entryId: z.string() }),
]);

export const manualSortLayoutSchema = z.object({ version: z.literal(1), items: z.array(manualSortItemSchema) });

export const corpusPartOfSpeechSchema = z.enum([
  "noun", "verb", "adjective", "adverb", "pronoun", "particle", "other",
]);

export const fontPresetSchema = z.enum([
  "auto", "charisSil", "notoSerif", "notoSerifCjkTc", "chironSungHk", "chironHeiHk",
]);

export const exportSettingsSchema = z.object({
  version: z.literal(1),
  corpus: z.object({
    partOfSpeechMappings: z.record(z.string(), corpusPartOfSpeechSchema),
  }),
  latex: z.object({
    title: z.string(),
    author: z.string(),
    headwordWritingSystemId: z.string(),
    pronunciationWritingSystemId: nullableText,
    exampleWritingSystemId: z.string(),
    collationLanguageTag: nullableText,
    sectionMode: z.enum(["auto", "firstGrapheme", "none"]),
    reverseIndex: z.enum(["gloss", "none"]),
    relatedEntries: z.enum(["none", "root", "base", "both"]),
    fontPresets: z.record(z.string(), fontPresetSchema),
  }),
});

export const exportKindSchema = z.enum(["corpusCsv", "latex", "pdf"]);
export const exportIssueSchema = z.object({
  severity: z.enum(["error", "warning"]),
  code: z.string(),
  entryId: nullableText,
  senseId: nullableText,
  field: nullableText,
  details: nullableText,
});
export const fontPackStatusSchema = z.object({
  id: z.string(),
  version: z.string(),
  state: z.enum(["missing", "installed", "invalid"]),
  mandatory: z.boolean(),
  installedBytes: z.number(),
});
export const exportPreviewSchema = z.object({
  snapshotToken: z.string(),
  rowCount: z.number(),
  issues: z.array(exportIssueSchema),
  omitted: z.object({
    examples: z.number(),
    exampleForms: z.number(),
    baseRelations: z.number(),
  }),
  requiredFontPacks: z.array(fontPackStatusSchema),
});
export const exportResultSchema = z.object({
  csvPath: nullableText,
  latexDirectory: nullableText,
  zipPath: nullableText,
  pdfPath: nullableText,
  pdfStatus: z.enum(["notRequested", "created", "xeLatexMissing", "failed"]),
  rowCount: z.number(),
  issues: z.array(exportIssueSchema),
  diagnosticPath: nullableText,
});
export const texEngineStatusSchema = z.object({
  available: z.boolean(),
  path: nullableText,
});

export const projectSchema = z.object({
  id: z.string(),
  name: z.string(),
  languageName: nullableText,
  languageCode: nullableText,
  analysisLanguage: z.enum(["zh-TW", "en"]).nullable(),
  description: nullableText,
  createdAt: z.string(),
  updatedAt: z.string(),
});

export const projectSnapshotSchema = z.object({
  rootPath: z.string(),
  project: projectSchema,
  writingSystems: z.array(writingSystemSchema),
  partOfSpeechOptions: z.array(z.string()),
  semanticDomainOptions: z.array(z.string()),
  exportSettings: exportSettingsSchema,
  entrySortSettings: entrySortSettingsSchema,
  manualSortLayout: manualSortLayoutSchema,
  entries: z.array(entrySummarySchema),
});

export const deletedEntrySchema = z.object({
  id: z.string(),
  deletedAt: z.string(),
});

export type WritingSystem = z.infer<typeof writingSystemSchema>;
export type EntryForm = z.infer<typeof entryFormSchema>;
export type ExampleForm = z.infer<typeof exampleFormSchema>;
export type Example = z.infer<typeof exampleSchema>;
export type Sense = z.infer<typeof senseSchema>;
export type EntryRelation = z.infer<typeof relationSchema>;
export type LexicalEntry = z.infer<typeof lexicalEntrySchema>;
export type EntrySummary = z.infer<typeof entrySummarySchema>;
export type EntrySortSettings = z.infer<typeof entrySortSettingsSchema>;
export type ManualSortItem = z.infer<typeof manualSortItemSchema>;
export type ManualSortLayout = z.infer<typeof manualSortLayoutSchema>;
export type Project = z.infer<typeof projectSchema>;
export type ProjectSnapshot = z.infer<typeof projectSnapshotSchema>;
export type CorpusPartOfSpeech = z.infer<typeof corpusPartOfSpeechSchema>;
export type ExportSettings = z.infer<typeof exportSettingsSchema>;
export type ExportKind = z.infer<typeof exportKindSchema>;
export type ExportIssue = z.infer<typeof exportIssueSchema>;
export type ExportPreview = z.infer<typeof exportPreviewSchema>;
export type ExportResult = z.infer<typeof exportResultSchema>;
export type TexEngineStatus = z.infer<typeof texEngineStatusSchema>;
export type FontPackStatus = z.infer<typeof fontPackStatusSchema>;

export function createId(): string {
  return crypto.randomUUID();
}
