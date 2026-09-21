(() => {
  "use strict";
  const $ = (selector) => document.querySelector(selector);
  const els = { title: $("#site-title"), list: $("#entry-list"), count: $("#entry-count"), search: $("#search-input"), state: $("#detail-state"), content: $("#detail-content"), detail: $("#entry-detail"), theme: $("#theme-button"), info: $("#info-button"), infoDialog: $("#info-dialog"), infoContent: $("#info-content"), back: $("#back-button") };
  let corpus = null, selected = null, filtered = [], activeAudio = null, listScroll = 0;
  const text = {
    "zh-TW": { search: "搜尋詞形、讀音或定義", entries: (n) => `${n} 個詞項`, empty: "找不到符合的詞項", missing: "找不到這個詞項", failed: "辭典資料無法載入，請稍後再試。", image: "圖片無法載入", audio: "音檔無法播放", play: "播放", pause: "暫停", about: "關於本辭典", close: "關閉", back: "返回詞項列表", dark: "切換為深色模式", light: "切換為亮色模式", notes: "備註", relations: "相關詞項", root: "詞根", base: "基底" },
    en: { search: "Search forms, pronunciation or definitions", entries: (n) => `${n} entries`, empty: "No matching entries", missing: "This entry was not found", failed: "The dictionary could not be loaded. Try again later.", image: "Image unavailable", audio: "Audio unavailable", play: "Play", pause: "Pause", about: "About this dictionary", close: "Close", back: "Back to entry list", dark: "Use dark theme", light: "Use light theme", notes: "Notes", relations: "Related entries", root: "Root", base: "Base" }
  };
  let locale = "zh-TW";
  const t = (key) => text[locale]?.[key] ?? text.en[key] ?? key;
  const escape = (value) => String(value ?? "").replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]);
  const fold = (value) => String(value ?? "").normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase(locale);
  const mediaUrl = (path) => new URL(path, new URL(corpus.site.mediaBaseUrl, location.href)).href;
  const systemName = (id) => corpus.writingSystems.find((item) => item.id === id)?.name ?? id;
  const allText = (entry) => [entry.primaryForm, entry.notes, ...entry.forms.map((f) => f.text), ...entry.senses.flatMap((s) => [s.partOfSpeech, s.gloss, s.definition, s.semanticDomain, ...s.examples.flatMap((e) => [e.translation, e.notes, ...e.forms.map((f) => f.text)])])].join(" ");
  function applyTheme(value) {
    const dark = value === "dark" || (value === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    els.theme.setAttribute("aria-label", dark ? t("light") : t("dark"));
  }
  function setupTheme() {
    applyTheme(localStorage.getItem("bkuw-theme") || "system");
    els.theme.addEventListener("click", () => { const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark"; localStorage.setItem("bkuw-theme", next); applyTheme(next); });
    matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", () => { if (!localStorage.getItem("bkuw-theme")) applyTheme("system"); });
  }
  function renderList() {
    els.count.textContent = t("entries")(filtered.length);
    if (!filtered.length) { els.list.innerHTML = `<div class="state-message">${escape(t("empty"))}</div>`; return; }
    els.list.innerHTML = filtered.map((entry) => {
      const sense = entry.senses[0] || {};
      const summary = [sense.partOfSpeech, sense.gloss || sense.definition].filter(Boolean).join(" · ");
      return `<button class="entry-item" type="button" role="option" data-slug="${escape(entry.slug)}" aria-selected="${entry.slug === selected}"><span class="entry-headword">${escape(entry.primaryForm)}</span><span class="entry-summary">${escape(summary)}</span></button>`;
    }).join("");
  }
  function renderAudio(items) {
    if (!items?.length) return "";
    return `<div class="audio-list">${items.map((item) => `<div class="audio-player" data-audio><button type="button" aria-label="${escape(t("play"))}">▶</button><input type="range" min="0" max="1000" value="0" aria-label="${escape(item.originalFilename || t("play"))}"><span class="audio-time">0:00</span><audio preload="metadata" src="${escape(mediaUrl(item.path))}"></audio></div>`).join("")}</div>`;
  }
  function renderEntry(entry) {
    const forms = entry.forms.filter((form) => form.text && form.text !== entry.primaryForm).map((form) => `<span><span class="form-label">${escape(systemName(form.writingSystemId))}</span> ${escape(form.text)}</span>`).join("");
    const senses = entry.senses.map((sense, index) => `<section class="sense">
      <div class="sense-heading"><span class="sense-number">${index + 1}</span>${sense.partOfSpeech ? `<span class="pos">${escape(sense.partOfSpeech)}</span>` : ""}${sense.semanticDomain ? `<span class="domain">${escape(sense.semanticDomain)}</span>` : ""}</div>
      ${sense.gloss ? `<p class="gloss">${escape(sense.gloss)}</p>` : ""}${sense.definition ? `<p class="definition">${escape(sense.definition)}</p>` : ""}
      ${sense.images?.length ? `<div class="media-grid">${sense.images.map((image) => `<figure class="media-frame"><img src="${escape(mediaUrl(image.path))}" alt="" loading="lazy"><div class="media-error" hidden>${escape(t("image"))}</div></figure>`).join("")}</div>` : ""}
      ${renderAudio(sense.audio)}
      ${sense.examples?.length ? `<ol class="examples">${sense.examples.map((example) => `<li class="example">${example.forms.map((form) => `<p class="example-form"><span class="sr-only">${escape(systemName(form.writingSystemId))}: </span>${escape(form.text)}</p>`).join("")}${example.translation ? `<div class="example-translation">${escape(example.translation)}</div>` : ""}${example.notes ? `<div class="example-notes"><strong>${escape(t("notes"))}:</strong> ${escape(example.notes)}</div>` : ""}${renderAudio(example.audio)}</li>`).join("")}</ol>` : ""}
    </section>`).join("");
    const relations = entry.relations?.length ? `<section class="relations"><h2>${escape(t("relations"))}</h2><ul>${entry.relations.map((relation) => `<li>${escape(t(relation.type))}: ${relation.targetSlug ? `<a href="#/entry/${encodeURIComponent(relation.targetSlug)}">${escape(relation.targetHeadword || relation.fallbackText)}</a>` : escape(relation.fallbackText || relation.targetHeadword || "")}</li>`).join("")}</ul></section>` : "";
    els.content.innerHTML = `<div class="detail-inner"><h2 class="entry-title">${escape(entry.primaryForm)}</h2><div class="forms">${forms}</div>${entry.notes ? `<p class="entry-notes"><strong>${escape(t("notes"))}:</strong> ${escape(entry.notes)}</p>` : ""}${senses}${relations}</div>`;
    els.state.hidden = true; els.content.hidden = false;
    els.content.querySelectorAll("img").forEach((img) => img.addEventListener("error", () => { img.hidden = true; img.nextElementSibling.hidden = false; }));
    setupAudio();
  }
  function showSlug(slug, focus = false) {
    const entry = corpus.entries.find((item) => item.slug === slug);
    selected = entry?.slug ?? null; renderList();
    if (!entry) { els.content.hidden = true; els.state.hidden = false; els.state.textContent = t("missing"); return; }
    renderEntry(entry); document.body.classList.add("detail-open");
    if (focus) els.detail.focus();
  }
  function route() {
    const match = location.hash.match(/^#\/entry\/([^/]+)$/);
    const slug = match ? decodeURIComponent(match[1]) : corpus?.entries[0]?.slug;
    if (slug) showSlug(slug);
  }
  function setupAudio() {
    document.querySelectorAll("[data-audio]").forEach((player) => {
      const audio = player.querySelector("audio"), button = player.querySelector("button"), range = player.querySelector("input"), time = player.querySelector(".audio-time");
      const update = () => { range.value = audio.duration ? Math.round(audio.currentTime / audio.duration * 1000) : 0; time.textContent = Number.isFinite(audio.currentTime) ? `${Math.floor(audio.currentTime / 60)}:${String(Math.floor(audio.currentTime % 60)).padStart(2, "0")}` : "0:00"; };
      button.addEventListener("click", async () => { if (audio.paused) { if (activeAudio && activeAudio !== audio) activeAudio.pause(); activeAudio = audio; try { await audio.play(); } catch { button.textContent = "!"; button.setAttribute("aria-label", t("audio")); } } else audio.pause(); });
      audio.addEventListener("play", () => { button.textContent = "Ⅱ"; button.setAttribute("aria-label", t("pause")); });
      audio.addEventListener("pause", () => { button.textContent = "▶"; button.setAttribute("aria-label", t("play")); });
      audio.addEventListener("timeupdate", update); audio.addEventListener("ended", update); audio.addEventListener("error", () => { button.textContent = "!"; button.disabled = true; button.setAttribute("aria-label", t("audio")); });
      range.addEventListener("input", () => { if (audio.duration) audio.currentTime = Number(range.value) / 1000 * audio.duration; });
    });
  }
  async function init() {
    setupTheme();
    try {
      const response = await fetch("./data/corpus.json", { cache: "no-cache" }); if (!response.ok) throw new Error(String(response.status));
      corpus = await response.json(); locale = corpus.site.defaultLocale === "en" ? "en" : "zh-TW";
      applyTheme(localStorage.getItem("bkuw-theme") || "system");
      document.documentElement.lang = locale; document.title = corpus.site.title; els.title.textContent = corpus.site.title; els.search.placeholder = t("search"); els.back.setAttribute("aria-label", t("back")); $("#info-title").textContent = t("about"); $("#info-close").setAttribute("aria-label", t("close"));
      if (corpus.site.description) document.querySelector('meta[name="description"]').content = corpus.site.description;
      if (corpus.site.info?.html) { els.info.hidden = false; els.info.setAttribute("aria-label", t("about")); els.infoContent.innerHTML = corpus.site.info.html; }
      filtered = corpus.entries; renderList(); route();
    } catch { els.state.textContent = t("failed"); els.count.textContent = ""; }
  }
  els.search.addEventListener("input", () => { const query = fold(els.search.value.trim()); filtered = !query ? corpus.entries : corpus.entries.filter((entry) => fold(allText(entry)).includes(query)); renderList(); });
  els.list.addEventListener("click", (event) => { const button = event.target.closest("[data-slug]"); if (!button) return; listScroll = els.list.parentElement.scrollTop; location.hash = `#/entry/${encodeURIComponent(button.dataset.slug)}`; showSlug(button.dataset.slug, true); });
  els.back.addEventListener("click", () => { document.body.classList.remove("detail-open"); els.list.parentElement.scrollTop = listScroll; });
  els.info.addEventListener("click", () => els.infoDialog.showModal()); $("#info-close").addEventListener("click", () => els.infoDialog.close());
  addEventListener("hashchange", route); init();
})();
