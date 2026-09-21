import { fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import appScript from "../../../src-tauri/templates/publish-site/app.js?raw";
import pageHtml from "../../../src-tauri/templates/publish-site/index.html?raw";
import pageStyles from "../../../src-tauri/templates/publish-site/styles.css?raw";
import pageHeaders from "../../../src-tauri/templates/publish-site/_headers?raw";

const corpus = {
  schemaVersion: 1,
  site: { title: "Test Dictionary", description: "Test", defaultLocale: "en", defaultTheme: "system", mediaBaseUrl: "./media/", info: { html: "<h1>About</h1><p>Safe information.</p>" } },
  writingSystems: [{ id: "primary", name: "Orthography" }, { id: "ipa", name: "IPA" }],
  entries: [
    { id: "e1", slug: "guo", sectionLabel: "Custom first", primaryForm: "guò", notes: null, forms: [{ writingSystemId: "primary", text: "guò" }, { writingSystemId: "ipa", text: "kuɔ" }], senses: [{ id: "s1", partOfSpeech: "verb", gloss: "cross", definition: "move over", semanticDomain: "motion", images: [], audio: [], examples: [] }], relations: [] },
    { id: "e2", slug: "ata", sectionLabel: "Custom second", primaryForm: "ata", notes: null, forms: [{ writingSystemId: "primary", text: "ata" }], senses: [{ id: "s2", partOfSpeech: "noun", gloss: "person", definition: null, semanticDomain: null, images: [], audio: [], examples: [] }], relations: [] },
  ],
};

describe("published website template", () => {
  beforeEach(() => {
    document.documentElement.innerHTML = pageHtml;
    localStorage.clear(); history.replaceState(null, "", "/");
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() })));
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: true, json: async () => corpus })));
    HTMLDialogElement.prototype.showModal = function showModal() { this.open = true; };
    HTMLDialogElement.prototype.close = function close() { this.open = false; };
  });

  it("folds diacritics for search, preserves display text, deep-links, toggles theme, and shows optional info", async () => {
    (0, eval)(appScript);
    await waitFor(() => expect(document.querySelector("#site-title")?.textContent).toBe("Test Dictionary"));
    expect(document.body).not.toHaveTextContent("Loading dictionary");
    expect(pageStyles).toMatch(/\[hidden\]\s*\{\s*display:\s*none\s*!important;/);
    expect(pageHeaders).toContain("Cache-Control: public, max-age=0, must-revalidate");
    expect([...document.querySelectorAll(".entry-list-item strong")].map((item) => item.textContent)).toEqual(["guò", "ata"]);
    expect([...document.querySelectorAll(".entry-section-heading")].map((item) => item.textContent)).toEqual(["Custom first", "Custom second"]);
    const input = document.querySelector<HTMLInputElement>("#search-input")!;
    fireEvent.input(input, { target: { value: "guo" } });
    expect(document.querySelectorAll(".entry-list-item")).toHaveLength(1);
    expect(document.querySelector(".entry-list-item strong")?.textContent).toBe("guò");
    fireEvent.click(document.querySelector<HTMLButtonElement>(".entry-list-item")!);
    expect(location.hash).toBe("#/entry/guo");
    expect(document.querySelector("#entry-heading")?.textContent).toBe("guò");

    const theme = document.querySelector<HTMLButtonElement>("#theme-button")!;
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(theme.getAttribute("aria-label")).toBe("Switch to dark mode");
    fireEvent.click(theme);
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(theme.getAttribute("aria-label")).toBe("Switch to light mode");
    expect(localStorage.getItem("theme-mode")).toBe("dark");

    const info = document.querySelector<HTMLButtonElement>("#info-button")!;
    expect(info.hidden).toBe(false);
    fireEvent.click(info);
    expect(document.querySelector<HTMLDialogElement>("#info-dialog")?.open).toBe(true);
    expect(document.querySelector("#info-content h1")?.textContent).toBe("About");

    document.body.classList.add("mobile-detail");
    fireEvent.click(document.querySelector<HTMLButtonElement>("#back-button")!);
    expect(document.body).not.toHaveClass("mobile-detail");
    expect(document.activeElement).toBe(input);
  });
});
