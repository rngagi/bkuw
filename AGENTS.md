# bkuw repository instructions

## Product invariants

- Write the product name as `bkuw` everywhere.
- Keep the application local-first, single-user, offline-capable, and Unicode-native.
- SQLite is the canonical data store. The React frontend must never issue SQL directly.
- Keep lexical entries separate from their writing-system-specific forms.
- Store POS on senses only.
- Examples belong to senses and may have multiple writing-system-specific forms.
- Preserve user text in NFC. Derived search keys may fold case and diacritics but must never replace display text.
- The UI must remain complete in both English (`en`) and Taiwan Traditional Chinese (`zh-TW`).
- Milestone 1 excludes audio, exports, cloud services, authentication, collaboration, and mobile clients.

## Architecture and security

- Put project lifecycle, validation, migrations, backups, filesystem access, and SQLite transactions behind the Rust project/database module.
- Expose typed Tauri commands through the single frontend adapter in `src/lib/tauri.ts`; do not scatter `invoke` calls through UI modules.
- Save a lexical entry as one aggregate transaction, including forms, senses, examples, example forms, and relations.
- Keep Tauri capabilities narrow. Do not add shell, HTTP, or broad filesystem access without a reviewed requirement.
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
