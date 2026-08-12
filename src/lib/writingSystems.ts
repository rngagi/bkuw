import type { WritingSystem } from "../types/domain";

export function displayWritingSystemText(text: string, system?: WritingSystem): string {
  if (!text) return text;
  if (system?.type === "phonemic") return `/${text}/`;
  if (system?.type === "phonetic") return `[${text}]`;
  return text;
}
