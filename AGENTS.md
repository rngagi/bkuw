# bkuw repository instructions

## Product invariants

- Write the product name as `bkuw` everywhere.
- Keep the application local-first, single-user, offline-capable, and Unicode-native.
- SQLite is the canonical data store. The React frontend must never issue SQL directly.
- Keep lexical entries separate from their writing-system-specific forms.
- Store POS on senses only.
- Examples belong to senses and may have multiple writing-system-specific forms.
- Photos belong to senses. Accept PNG/JPEG/WebP input, lightly resize oversized images in the frontend, and store only validated project-local PNG files plus database metadata.
- Preserve user text in NFC. Derived search keys may fold case and diacritics but must never replace display text.
- The UI must remain complete in both English (`en`) and Taiwan Traditional Chinese (`zh-TW`).
- The current product includes corpus CSV and XeLaTeX/Overleaf/PDF export. Audio, cloud services, authentication, collaboration, mobile clients, production signing, notarization, and automatic upload remain excluded until explicitly planned.

## Architecture and security

- Put project lifecycle, validation, migrations, backups, filesystem access, and SQLite transactions behind the Rust project/database module.
- Keep sense-photo binary storage behind the Rust project/database module. Database paths must remain project-relative `media/images/<uuid>.png`; verify PNG decoding and SHA-256 before use or export.
- Expose typed Tauri commands through the single frontend adapter in `src/lib/tauri.ts`; do not scatter `invoke` calls through UI modules.
- Save a lexical entry as one aggregate transaction, including forms, senses, examples, example forms, and relations.
- Keep Tauri capabilities narrow. Do not add shell, frontend HTTP, or broad filesystem access without a reviewed requirement. The Rust font manager may download only fixed catalog URLs and must verify SHA-256 before activation.
- Keep all export validation, sorting, escaping, filesystem writes, font-pack resolution, and XeLaTeX process execution behind the Rust export/font-manager modules. React may only call their typed commands through `src/lib/tauri.ts`.
- Portable LaTeX/PDF exports must use bkuw-managed fonts, include the used font files and licenses, and never depend on system-installed fonts. TeX Gyre Termes is mandatory; phonemic/phonetic writing systems always use Charis SIL.
- Treat the rngagi-corpus v0.3 nine-column order as a versioned external contract. This repository currently has no cross-repository automated contract test; corpus changes require manual revalidation and a golden-fixture update.
- CI and packaging support Windows x64 and macOS Apple Silicon only. Do not add macOS Intel targets.
- Run validation and platform packaging on `main`; version-tag release workflows must promote artifacts from a successful `main` CI run for the exact same commit instead of rebuilding them.
- Prefer a small module interface with substantial behavior behind it. Avoid pass-through modules and speculative seams.

## Development workflow

- Use pnpm for JavaScript dependencies and commands. Do not add npm or yarn lockfiles.
- Add a migration for every schema change; never edit an already-released migration.
- Add or update tests with behavior changes. Test through public module interfaces rather than implementation details.
- Run `pnpm check`, `pnpm test`, and `pnpm test:rust` before marking an implementation checklist item complete.
- Run `pnpm tauri build --no-bundle` before marking the milestone complete.
- Update `plan.md`, `docs/product-spec.md`, and `docs/architecture.md` when their corresponding behavior changes.
- Only check an item in `plan.md` after its stated verification succeeds.

## UI rules

- Use shadcn/ui conventions, Radix primitives, Tailwind CSS, and Lucide icons only.
- Keep the workspace flat, restrained, keyboard-efficient, and suitable for long data-entry sessions.
- Never use gradients, glassmorphism, oversized cards, or decorative animation.
- Use the semantic muted-red primary token; do not hard-code the brand color throughout components.
- User-facing strings, validation messages, errors, empty states, and confirmations must use translation keys.
