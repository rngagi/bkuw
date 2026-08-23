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

export const senseImageSchema = z.object({
  id: z.string(),
  senseId: z.string(),
  originalFilename: z.string(),
  width: z.number().int().positive(),
  height: z.number().int().positive(),
  byteSize: z.number().int().positive(),
  sortOrder: z.number().int(),
  createdAt: z.string(),
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
  version: z.literal(2),
  mode: z.enum(["auto", "manual"]),
  source: z.enum(["writingSystem", "semanticDomain"]),
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
    includeSenseImages: z.boolean(),
    includeSemanticDomains: z.boolean().default(true),
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
export const fontInstallProgressSchema = z.object({
  packId: z.string(),
  phase: z.enum(["downloading", "verifying", "installed", "failed"]),
  packIndex: z.number().int().nonnegative(),
  packCount: z.number().int().nonnegative(),
  downloadedBytes: z.number().nonnegative(),
  totalBytes: z.number().nonnegative().nullable(),
});
export const csvDelimiterSchema = z.enum(["comma", "tab", "semicolon"]);
export const csvColumnSchema = z.object({ index: z.number().int().nonnegative(), name: z.string(), samples: z.array(z.string()) });
export const csvInspectionSchema = z.object({
  sourcePath: z.string(), fileName: z.string(), sha256: z.string(), delimiter: csvDelimiterSchema,
  rowCount: z.number().int().nonnegative(), columns: z.array(csvColumnSchema), profile: nullableText,
});
export const csvMappingTargetSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("ignore") }),
  z.object({ kind: z.literal("entryForm"), writingSystemId: z.string() }),
  z.object({ kind: z.literal("entryNotes") }),
  z.object({ kind: z.literal("senseGloss") }),
  z.object({ kind: z.literal("senseDefinition") }),
  z.object({ kind: z.literal("partOfSpeech") }),
  z.object({ kind: z.literal("semanticDomain") }),
  z.object({ kind: z.literal("exampleForm"), writingSystemId: z.string() }),
  z.object({ kind: z.literal("exampleTranslation") }),
  z.object({ kind: z.literal("exampleNotes") }),
  z.object({ kind: z.literal("rootFallback") }),
]);
export const csvColumnMappingSchema = z.object({ columnIndex: z.number().int().nonnegative(), target: csvMappingTargetSchema });
export const csvImportGroupSchema = z.object({ rowIndices: z.array(z.number().int().nonnegative()) });
export const csvProjectSpecSchema = z.object({
  parentDir: z.string(), name: z.string(), languageName: nullableText, languageCode: nullableText,
  analysisLanguage: z.enum(["zh-TW", "en"]).nullable(), writingSystems: z.array(writingSystemSchema),
});
export const csvPreviewRequestSchema = z.object({
  sourcePath: z.string(), delimiter: csvDelimiterSchema, project: csvProjectSpecSchema,
  mappings: z.array(csvColumnMappingSchema), groups: z.array(csvImportGroupSchema),
  excludedRows: z.array(z.number().int().nonnegative()), rootDelimiter: z.string(),
});
export const csvPreviewIssueSchema = z.object({
  severity: z.enum(["error", "warning"]), code: z.string(),
  rowIndices: z.array(z.number().int().nonnegative()),
  columnIndices: z.array(z.number().int().nonnegative()).default([]), details: nullableText,
});
export const csvImportPreviewSchema = z.object({
  previewToken: z.string(), sourceRowCount: z.number().int().nonnegative(), importEntryCount: z.number().int().nonnegative(),
  importSenseCount: z.number().int().nonnegative(), skippedRowCount: z.number().int().nonnegative(),
  blockingErrorCount: z.number().int().nonnegative(), warningCount: z.number().int().nonnegative(),
  issues: z.array(csvPreviewIssueSchema), groups: z.array(z.object({ rowIndices: z.array(z.number().int().nonnegative()), primaryForm: z.string(), blocked: z.boolean() })),
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

export const senseImageMutationSchema = z.object({
  entry: lexicalEntrySchema,
  image: senseImageSchema.nullable(),
});

export const senseImageContentSchema = z.object({
  mimeType: z.literal("image/png"),
  dataBase64: z.string(),
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
export const csvImportResultSchema = z.object({
  snapshot: projectSnapshotSchema, importedEntryCount: z.number().int().nonnegative(),
  importedSenseCount: z.number().int().nonnegative(), skippedRowCount: z.number().int().nonnegative(),
  warnings: z.array(csvPreviewIssueSchema),
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
export type SenseImage = z.infer<typeof senseImageSchema>;
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
export type FontInstallProgress = z.infer<typeof fontInstallProgressSchema>;
export type CsvDelimiter = z.infer<typeof csvDelimiterSchema>;
export type CsvInspection = z.infer<typeof csvInspectionSchema>;
export type CsvMappingTarget = z.infer<typeof csvMappingTargetSchema>;
export type CsvColumnMapping = z.infer<typeof csvColumnMappingSchema>;
export type CsvImportGroup = z.infer<typeof csvImportGroupSchema>;
export type CsvProjectSpec = z.infer<typeof csvProjectSpecSchema>;
export type CsvPreviewRequest = z.infer<typeof csvPreviewRequestSchema>;
export type CsvPreviewIssue = z.infer<typeof csvPreviewIssueSchema>;
export type CsvImportPreview = z.infer<typeof csvImportPreviewSchema>;
export type CsvImportResult = z.infer<typeof csvImportResultSchema>;

export function createId(): string {
  return crypto.randomUUID();
}
