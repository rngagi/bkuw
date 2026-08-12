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
  partsOfSpeech: z.array(z.string()),
  revision: z.number(),
});

export const projectSchema = z.object({
  id: z.string(),
  name: z.string(),
  languageName: nullableText,
  languageCode: nullableText,
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
export type Project = z.infer<typeof projectSchema>;
export type ProjectSnapshot = z.infer<typeof projectSnapshotSchema>;

export function createId(): string {
  return crypto.randomUUID();
}
