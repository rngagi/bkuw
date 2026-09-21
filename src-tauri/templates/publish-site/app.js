"use strict";

const STORAGE = {
  theme: "theme-mode",
};

const messages = {
  "zh-TW": {
    switchToLight: "切換至亮色模式",
    switchToDark: "切換至深色模式",
    aboutDictionary: "關於本辭典",
    close: "關閉",
    search: "搜尋詞項",
    searchPlaceholder: "搜尋詞形或釋義",
    backToList: "返回詞項列表",
    loading: "正在載入辭典…",
    loadError: "無法載入辭典資料。請確認本機伺服器與 data/corpus.json。",
    entriesCount: "{{shown}} / {{total}} 個詞項",
    noResults: "找不到符合「{{query}}」的詞項",
    noSelection: "請從左側選擇詞項",
    dictionaryEntry: "詞項",
    unnamedSense: "未命名義項",
    semanticDomain: "語意類別：{{value}}",
    audio: "音檔",
    senseAudio: "義項錄音 {{number}}",
    exampleAudio: "例句錄音 {{number}}",
    playAudio: "播放{{label}}",
    pauseAudio: "暫停{{label}}",
    seekAudio: "調整{{label}}播放位置",
    audioUnavailable: "無法播放這個音檔",
    images: "圖片",
    imageAlt: "{{headword}}第 {{sense}} 個義項的圖片 {{number}}",
    imageUnavailable: "無法載入圖片",
    examples: "例句",
    exampleNumber: "例句 {{number}}",
    translation: "翻譯",
    notes: "備註",
    relations: "相關詞項",
    root: "詞根",
    base: "詞基",
    poweredBy: "使用 bkuw 建立 & 發布",
  },
  en: {
    switchToLight: "Switch to light mode",
    switchToDark: "Switch to dark mode",
    aboutDictionary: "About this dictionary",
    close: "Close",
    search: "Search entries",
    searchPlaceholder: "Search forms or definitions",
    backToList: "Back to entries",
    loading: "Loading dictionary…",
    loadError: "The dictionary data could not be loaded. Check the local server and data/corpus.json.",
    entriesCount: "{{shown}} of {{total}} entries",
    noResults: "No entries match “{{query}}”",
    noSelection: "Choose an entry from the list",
    dictionaryEntry: "Dictionary entry",
    unnamedSense: "Unnamed sense",
    semanticDomain: "Semantic domain: {{value}}",
    audio: "Audio",
    senseAudio: "Sense recording {{number}}",
    exampleAudio: "Example recording {{number}}",
    playAudio: "Play {{label}}",
    pauseAudio: "Pause {{label}}",
    seekAudio: "Seek {{label}}",
    audioUnavailable: "This audio file could not be played",
    images: "Images",
    imageAlt: "Image {{number}} for sense {{sense}} of {{headword}}",
    imageUnavailable: "Image unavailable",
    examples: "Examples",
    exampleNumber: "Example {{number}}",
    translation: "Translation",
    notes: "Notes",
    relations: "Related entries",
    root: "Root",
    base: "Base",
    poweredBy: "Built & published with bkuw",
  },
};

const state = {
  corpus: null,
  locale: "zh-TW",
  themeMode: getStored(STORAGE.theme, "system"),
  query: "",
  selectedId: null,
  activeAudio: null,
};

const elements = {
  workspace: document.querySelector(".workspace"),
  siteTitle: document.querySelector("#site-title"),
  theme: document.querySelector("#theme-button"),
  themeMoon: document.querySelector(".theme-icon-moon"),
  themeSun: document.querySelector(".theme-icon-sun"),
  info: document.querySelector("#info-button"),
  infoDialog: document.querySelector("#info-dialog"),
  infoClose: document.querySelector("#info-close-button"),
  infoContent: document.querySelector("#info-content"),
  search: document.querySelector("#search-input"),
  summary: document.querySelector("#result-summary"),
  list: document.querySelector("#entry-list"),
  detail: document.querySelector("#entry-detail"),
  back: document.querySelector("#back-button"),
  announcer: document.querySelector("#announcer"),
  themeColor: document.querySelector('meta[name="theme-color"]'),
};

const systemTheme = matchMedia("(prefers-color-scheme: dark)");

function getStored(key, fallback) {
  try { return localStorage.getItem(key) || fallback; } catch { return fallback; }
}

function store(key, value) {
  try { localStorage.setItem(key, value); } catch { /* Local storage may be disabled. */ }
}

function t(key, values = {}) {
  const template = messages[state.locale]?.[key] ?? messages["zh-TW"][key] ?? key;
  return Object.entries(values).reduce(
    (result, [name, value]) => result.replaceAll(`{{${name}}}`, String(value)),
    template,
  );
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function fold(value) {
  return String(value ?? "")
    .normalize("NFD")
    .replace(/\p{Mark}/gu, "")
    .toLocaleLowerCase();
}

function resolvedTheme() {
  if (state.themeMode === "light" || state.themeMode === "dark") return state.themeMode;
  return systemTheme.matches ? "dark" : "light";
}

function applyTheme() {
  const theme = resolvedTheme();
  document.documentElement.dataset.theme = theme;
  elements.themeColor.content = theme === "dark" ? "#151816" : "#f7f8f6";
  const target = theme === "dark" ? "light" : "dark";
  const label = t(target === "light" ? "switchToLight" : "switchToDark");
  elements.theme.setAttribute("aria-label", label);
  elements.theme.title = label;
  elements.themeMoon.toggleAttribute("hidden", target !== "dark");
  elements.themeSun.toggleAttribute("hidden", target !== "light");
}

function applyLocalization() {
  document.documentElement.lang = state.locale === "zh-TW" ? "zh-Hant" : "en";
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  elements.search.placeholder = t("searchPlaceholder");
  elements.search.setAttribute("aria-label", t("search"));
  elements.info.setAttribute("aria-label", t("aboutDictionary"));
  elements.info.title = t("aboutDictionary");
  elements.infoDialog.setAttribute("aria-label", t("aboutDictionary"));
  elements.infoClose.setAttribute("aria-label", t("close"));
  elements.infoClose.title = t("close");
  applyTheme();
}

function searchableText(entry) {
  return [
    entry.primaryForm,
    entry.notes,
    ...entry.forms.map((form) => form.text),
    ...entry.senses.flatMap((sense) => [
      sense.partOfSpeech,
      sense.gloss,
      sense.definition,
      sense.semanticDomain,
      ...sense.examples.flatMap((example) => [
        example.translation,
        example.notes,
        ...example.forms.map((form) => form.text),
      ]),
    ]),
    ...(entry.relations || []).flatMap((relation) => [relation.targetHeadword, relation.fallbackText]),
  ].filter(Boolean).join(" ");
}

function filteredEntries() {
  if (!state.corpus) return [];
  const query = fold(state.query.trim());
  if (!query) return state.corpus.entries;
  return state.corpus.entries.filter((entry) => fold(searchableText(entry)).includes(query));
}

function sectionLabel(entry) {
  return entry.sectionLabel || Array.from(entry.primaryForm.trim())[0]?.toLocaleUpperCase() || "#";
}

function mediaUrl(path) {
  const base = new URL(state.corpus.site.mediaBaseUrl, location.href);
  return new URL(path, base).href;
}

function formatTime(seconds) {
  const safe = Number.isFinite(seconds) && seconds > 0 ? seconds : 0;
  return `${Math.floor(safe / 60)}:${String(Math.floor(safe % 60)).padStart(2, "0")}`;
}

function playIcon() {
  return '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="m8 5 11 7-11 7Z"/></svg>';
}

function pauseIcon() {
  return '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M9 5v14M15 5v14"/></svg>';
}

function renderAudio(items, kind) {
  if (!items.length) return "";
  return `
    <section class="media-section audio-section" aria-label="${escapeHtml(t("audio"))}">
      <h3>${escapeHtml(t("audio"))}</h3>
      <div class="audio-list">
        ${items.map((item, index) => {
          const label = t(kind === "example" ? "exampleAudio" : "senseAudio", { number: index + 1 });
          const duration = item.durationMs / 1000;
          return `
            <div class="audio-player" data-audio-label="${escapeHtml(label)}">
              <audio preload="none" src="${escapeHtml(mediaUrl(item.path))}"></audio>
              <button class="audio-toggle" type="button" aria-label="${escapeHtml(t("playAudio", { label }))}">${playIcon()}</button>
              <div class="audio-name"><strong>${escapeHtml(label)}</strong><small>${escapeHtml(formatTime(duration))}</small></div>
              <input class="audio-seek" type="range" min="0" max="${duration}" step="0.01" value="0" aria-label="${escapeHtml(t("seekAudio", { label }))}" />
              <time class="audio-time">0:00 / ${escapeHtml(formatTime(duration))}</time>
              <span class="audio-error" role="alert" hidden>${escapeHtml(t("audioUnavailable"))}</span>
            </div>
          `;
        }).join("")}
      </div>
    </section>
  `;
}

function renderImages(images, entry, senseIndex) {
  if (!images.length) return "";
  return `
    <section class="media-section image-section" aria-label="${escapeHtml(t("images"))}">
      <h3>${escapeHtml(t("images"))}</h3>
      <div class="image-grid">
        ${images.map((image, index) => {
          const alt = t("imageAlt", { headword: entry.primaryForm, sense: senseIndex + 1, number: index + 1 });
          return `
            <figure class="sense-image">
              <div class="image-frame">
                <img src="${escapeHtml(mediaUrl(image.path))}" alt="${escapeHtml(alt)}" width="${image.width}" height="${image.height}" loading="lazy" decoding="async" />
                <span class="image-fallback" role="status" hidden>${escapeHtml(t("imageUnavailable"))}</span>
              </div>
              <figcaption>${escapeHtml(image.originalFilename)}</figcaption>
            </figure>
          `;
        }).join("")}
      </div>
    </section>
  `;
}

function renderExamples(examples, writingSystems) {
  if (!examples.length) return "";
  return `
    <section class="examples-section" aria-label="${escapeHtml(t("examples"))}">
      <h3>${escapeHtml(t("examples"))}</h3>
      <ol class="example-list">
        ${examples.map((example, exampleIndex) => `
          <li class="example">
            <span class="example-label">${escapeHtml(t("exampleNumber", { number: exampleIndex + 1 }))}</span>
            <div class="example-forms">
              ${example.forms.map((form) => `
                <div class="example-form">
                  <p lang="${escapeHtml(writingSystems.get(form.writingSystemId)?.languageTag || "")}">${escapeHtml(form.text)}</p>
                  ${example.forms.length > 1 ? `<small>${escapeHtml(writingSystems.get(form.writingSystemId)?.name || "")}</small>` : ""}
                </div>
              `).join("")}
            </div>
            ${example.translation ? `<p class="example-translation"><span>${escapeHtml(t("translation"))}</span>${escapeHtml(example.translation)}</p>` : ""}
            ${example.notes ? `<p class="published-notes"><strong>${escapeHtml(t("notes"))}</strong>${escapeHtml(example.notes)}</p>` : ""}
            ${renderAudio(example.audio, "example")}
          </li>
        `).join("")}
      </ol>
    </section>
  `;
}

function renderList() {
  if (!state.corpus) return;
  const entries = filteredEntries();
  elements.summary.textContent = t("entriesCount", { shown: entries.length, total: state.corpus.entries.length });
  if (!entries.length) {
    elements.list.innerHTML = `<div class="state-panel"><p>${escapeHtml(t("noResults", { query: state.query }))}</p></div>`;
    return;
  }

  let previousSection = null;
  elements.list.innerHTML = entries.map((entry) => {
    const currentSection = sectionLabel(entry);
    const heading = currentSection !== previousSection
      ? `<h2 class="entry-section-heading">${escapeHtml(currentSection)}</h2>`
      : "";
    previousSection = currentSection;
    const summaries = entry.senses.slice(0, 2).map((sense) => {
      const prefix = sense.partOfSpeech ? `${sense.partOfSpeech} · ` : "";
      return `${prefix}${sense.gloss || sense.definition || t("unnamedSense")}`;
    }).join("；");
    return `${heading}<button class="entry-list-item" type="button" data-entry-id="${escapeHtml(entry.id)}" aria-current="${entry.id === state.selectedId}"><strong>${escapeHtml(entry.primaryForm || "…")}</strong><small>${escapeHtml(summaries)}</small></button>`;
  }).join("");

  elements.list.querySelectorAll("[data-entry-id]").forEach((button) => {
    button.addEventListener("click", () => navigateToEntry(button.dataset.entryId, true));
  });
}

function renderDetail() {
  if (!state.corpus) return;
  const entry = state.corpus.entries.find((item) => item.id === state.selectedId);
  if (!entry) {
    elements.detail.innerHTML = `<div class="state-panel"><p>${escapeHtml(t("noSelection"))}</p></div>`;
    return;
  }

  const writingSystems = new Map(state.corpus.writingSystems.map((system) => [system.id, system]));
  const secondaryForms = entry.forms.filter((form) => form.text !== entry.primaryForm);
  const forms = secondaryForms.length
    ? `<dl class="entry-forms">${secondaryForms.map((form) => `<div class="entry-form"><dt>${escapeHtml(writingSystems.get(form.writingSystemId)?.name || "")}</dt><dd>${escapeHtml(form.text)}</dd></div>`).join("")}</dl>`
    : "";
  const senses = entry.senses.map((sense, senseIndex) => `
    <article class="sense">
      <span class="sense-number" aria-hidden="true"></span>
      <div class="sense-content">
        <div class="sense-heading">
          <h2>${escapeHtml(sense.gloss || sense.definition || t("unnamedSense"))}</h2>
          ${sense.partOfSpeech ? `<span class="pos-tag">${escapeHtml(sense.partOfSpeech)}</span>` : ""}
        </div>
        ${sense.definition && sense.definition !== sense.gloss ? `<p class="definition">${escapeHtml(sense.definition)}</p>` : ""}
        ${sense.semanticDomain ? `<p class="semantic-domain">${escapeHtml(t("semanticDomain", { value: sense.semanticDomain }))}</p>` : ""}
        ${renderAudio(sense.audio, "sense")}
        ${renderImages(sense.images, entry, senseIndex)}
        ${renderExamples(sense.examples, writingSystems)}
      </div>
    </article>
  `).join("");
  const relations = entry.relations?.length
    ? `<section class="relations-section"><h2>${escapeHtml(t("relations"))}</h2><ul>${entry.relations.map((relation) => {
      const label = t(relation.type);
      const value = relation.targetHeadword || relation.fallbackText || "";
      const target = relation.targetSlug
        ? `<a href="#/entry/${encodeURIComponent(relation.targetSlug)}">${escapeHtml(value)}</a>`
        : escapeHtml(value);
      return `<li><strong>${escapeHtml(label)}</strong>${target}</li>`;
    }).join("")}</ul></section>`
    : "";

  elements.detail.innerHTML = `
    <article class="entry-document">
      <header class="entry-header">
        <p class="entry-kicker">${escapeHtml(t("dictionaryEntry"))}</p>
        <h1 id="entry-heading">${escapeHtml(entry.primaryForm || "…")}</h1>
        ${forms}
        ${entry.notes ? `<p class="published-notes"><strong>${escapeHtml(t("notes"))}</strong>${escapeHtml(entry.notes)}</p>` : ""}
      </header>
      <div class="senses">${senses}</div>
      ${relations}
    </article>
  `;
  document.title = `${entry.primaryForm} · ${state.corpus.site.title}`;
  bindDetailMedia();
}

function resetAudioPlayer(player) {
  const audio = player.querySelector("audio");
  const button = player.querySelector(".audio-toggle");
  const label = player.dataset.audioLabel;
  button.innerHTML = playIcon();
  button.setAttribute("aria-label", t("playAudio", { label }));
  if (audio.ended) {
    audio.currentTime = 0;
    player.querySelector(".audio-seek").value = "0";
  }
}

function bindDetailMedia() {
  state.activeAudio = null;
  elements.detail.querySelectorAll(".audio-player").forEach((player) => {
    const audio = player.querySelector("audio");
    const button = player.querySelector(".audio-toggle");
    const seek = player.querySelector(".audio-seek");
    const time = player.querySelector(".audio-time");
    const error = player.querySelector(".audio-error");
    const label = player.dataset.audioLabel;
    const duration = Number(seek.max);

    button.addEventListener("click", async () => {
      error.hidden = true;
      if (!audio.paused) {
        audio.pause();
        return;
      }
      if (state.activeAudio && state.activeAudio !== player) {
        const previous = state.activeAudio.querySelector("audio");
        previous.pause();
        resetAudioPlayer(state.activeAudio);
      }
      state.activeAudio = player;
      try { await audio.play(); } catch { error.hidden = false; }
    });
    audio.addEventListener("play", () => {
      button.innerHTML = pauseIcon();
      button.setAttribute("aria-label", t("pauseAudio", { label }));
    });
    audio.addEventListener("pause", () => resetAudioPlayer(player));
    audio.addEventListener("ended", () => resetAudioPlayer(player));
    audio.addEventListener("timeupdate", () => {
      seek.value = String(audio.currentTime);
      time.textContent = `${formatTime(audio.currentTime)} / ${formatTime(duration)}`;
    });
    audio.addEventListener("error", () => {
      error.hidden = false;
      button.disabled = true;
    });
    seek.addEventListener("input", () => {
      audio.currentTime = Number(seek.value);
      time.textContent = `${formatTime(audio.currentTime)} / ${formatTime(duration)}`;
    });
  });

  elements.detail.querySelectorAll(".sense-image img").forEach((image) => {
    image.addEventListener("error", () => {
      image.hidden = true;
      image.nextElementSibling.hidden = false;
    });
  });
}

function entryFromHash() {
  const match = location.hash.match(/^#\/entry\/(.+)$/);
  if (!match || !state.corpus) return null;
  const slug = decodeURIComponent(match[1]);
  return state.corpus.entries.find((entry) => entry.slug === slug) ?? null;
}

function navigateToEntry(id, updateHash) {
  const entry = state.corpus?.entries.find((item) => item.id === id);
  if (!entry) return;
  state.selectedId = entry.id;
  if (updateHash && location.hash !== `#/entry/${encodeURIComponent(entry.slug)}`) {
    location.hash = `/entry/${encodeURIComponent(entry.slug)}`;
  }
  renderList();
  renderDetail();
  if (matchMedia("(max-width: 47.99rem)").matches) {
    document.body.classList.add("mobile-detail");
  }
}

function renderAll() {
  if (!state.corpus) return;
  elements.siteTitle.textContent = state.corpus.site.title;
  const infoHtml = state.corpus.site.info?.html?.trim() || "";
  elements.info.hidden = !infoHtml;
  elements.infoContent.innerHTML = infoHtml;
  renderList();
  renderDetail();
}

async function loadCorpus() {
  try {
    const response = await fetch("./data/corpus.json", { cache: "no-cache", headers: { Accept: "application/json" } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const corpus = await response.json();
    if (corpus.schemaVersion !== 1 || !Array.isArray(corpus.entries)) throw new Error("Unsupported corpus schema");
    state.corpus = corpus;
    state.locale = corpus.site.defaultLocale === "en" ? "en" : "zh-TW";
    applyLocalization();
    const linked = entryFromHash();
    state.selectedId = linked?.id || corpus.site.defaultEntryId || corpus.entries[0]?.id || null;
    if (linked && matchMedia("(max-width: 47.99rem)").matches) document.body.classList.add("mobile-detail");
    elements.workspace.setAttribute("aria-busy", "false");
    renderAll();
  } catch (error) {
    console.error(error);
    elements.workspace.setAttribute("aria-busy", "false");
    elements.detail.innerHTML = `<div class="state-panel" role="alert"><p>${escapeHtml(t("loadError"))}</p></div>`;
    elements.list.innerHTML = "";
  }
}

elements.search.addEventListener("input", (event) => {
  state.query = event.target.value;
  renderList();
});

elements.theme.addEventListener("click", () => {
  state.themeMode = resolvedTheme() === "dark" ? "light" : "dark";
  store(STORAGE.theme, state.themeMode);
  applyTheme();
});

systemTheme.addEventListener("change", () => {
  if (state.themeMode === "system") applyTheme();
});

elements.info.addEventListener("click", () => {
  if (typeof elements.infoDialog.showModal === "function") elements.infoDialog.showModal();
  else elements.infoDialog.setAttribute("open", "");
});

elements.infoClose.addEventListener("click", () => {
  if (typeof elements.infoDialog.close === "function") elements.infoDialog.close();
  else elements.infoDialog.removeAttribute("open");
});

elements.infoDialog.addEventListener("click", (event) => {
  if (event.target !== elements.infoDialog) return;
  const bounds = elements.infoDialog.getBoundingClientRect();
  const inside = event.clientX >= bounds.left && event.clientX <= bounds.right
    && event.clientY >= bounds.top && event.clientY <= bounds.bottom;
  if (!inside) elements.infoClose.click();
});

elements.back.addEventListener("click", () => {
  document.body.classList.remove("mobile-detail");
  elements.search.focus();
});

window.addEventListener("hashchange", () => {
  const entry = entryFromHash();
  if (entry) navigateToEntry(entry.id, false);
  else document.body.classList.remove("mobile-detail");
});

window.addEventListener("keydown", (event) => {
  const editing = event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || event.target instanceof HTMLTextAreaElement;
  if ((event.metaKey || event.ctrlKey) && event.key.toLocaleLowerCase() === "k") {
    event.preventDefault();
    elements.search.focus();
    elements.search.select();
  } else if (!editing && event.key === "/") {
    event.preventDefault();
    elements.search.focus();
  } else if (event.key === "Escape" && document.activeElement === elements.search && state.query) {
    elements.search.value = "";
    state.query = "";
    renderList();
  }
});

applyLocalization();
void loadCorpus();
