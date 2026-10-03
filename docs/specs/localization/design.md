# Localization: English, Portuguese and Japanese

Status: **built** — steps 1 to 8 written together on 2026-10-02, committed in `0cfc9ed`.
Date: 2026-10-02
Bevy: 0.19.1

## Goal

Every word the player reads is written in English inside the code: `button("New Game")`,
`const QUIT_QUESTION: &str = "Quit without saving?"`, `format!("{name} · {hours}h ...")`.
This design moves those words into one translation file per language, adds a Language choice
to the Settings overlay, and switches the whole UI live when it changes.

Three languages to start: English (`en`), Brazilian Portuguese (`pt-BR`) and Japanese (`ja`).
Japanese is in the first set on purpose: it is the one that breaks the easy assumptions (no
Latin letters in the font, no plurals, different word order), so the mechanism is proven
against the hard case now instead of when the fourth language arrives.

The terms: **i18n** (internationalization) is making the code able to speak any language,
**l10n** (localization) is writing the words for one. Steps 1 to 5 below are i18n; the
`.ftl` files are l10n.

## Decided

| Question | Decision |
| --- | --- |
| Library | Project Fluent (`fluent-bundle` + `unic-langid`), driven by our own `src/i18n.rs` |
| Translation files | `assets/locales/<code>/main.ftl`, embedded with `include_str!` |
| Languages | English, Português, 日本語; English is the fallback for any missing key |
| First run | Follows the OS language (`sys-locale`), English when it is not one of the three |
| Remembered | In `settings.ron`, next to fullscreen and volume |
| Where to choose | Settings overlay, a row of three radio buttons, each named in its own language |
| Switching | Live: every visible text changes at once, the menu or pause menu behind included |
| Font | Noto Sans JP replaces Bevy's default font for the whole UI |
| Player-typed text | Never translated: save names stay as typed |
| Japanese text | Ships as a draft written by Claude, not yet read by a native speaker |

## Out of scope

- **More languages.** Adding one is a `.ftl` file and a `Language` variant; that is the
  point of the design, not a task in it.
- **Right-to-left scripts** (Arabic, Hebrew). Parley shapes them, but the layout (button
  column on the left, scrim gradient) would need mirroring.
- **Translated images and audio** (a logo with words in it, voice lines). Only text.
- **Localized number and date formats** beyond what Fluent does on its own. Save ages are
  relative ("3d ago"), which keeps dates out entirely, as the save design chose.
- **Hot reloading the `.ftl` files.** They are compiled in; see "Why embedded".
- **A translation workflow** (Crowdin, Weblate, extracting keys). Three files edited by hand.
- **Translating the window title.** The game's name is a proper noun.

## The library

| Option | Version (crates.io, checked 2026-10-02) | For | Against |
| --- | --- | --- | --- |
| **`fluent-bundle`** (Project Fluent, Mozilla) | 0.16.0 | The standard format, used by Firefox; plurals and gender per language; arguments by name; a translator can reorder a sentence without touching code | We write the Bevy glue (~100 lines) ourselves |
| `bevy_fluent` | 0.15.0, needs `bevy ^0.19` | Fluent plus Bevy asset loading and hot reload | Pulls `serde_yaml` (deprecated), `uuid`, `futures-lite`, `indexmap`; the bundles load through the asset server, so the first frames have no text until they arrive, and every screen has to wait on that |
| `rust-i18n` | 4.2.4 | A `t!("key")` macro, YAML files, very little setup | The current language is a global, outside the ECS; only `%{name}` interpolation, no plural rules, so "1 saves" is a bug waiting in Portuguese |

**Proposed: `fluent-bundle` directly.** Fluent's plural selection is the feature that keeps
the translations honest:

```ftl
# en
save-played = { $hours }h { $minutes }m played
saves-count = { $count ->
    [one] 1 save
   *[other] { $count } saves
}
```

```ftl
# ja: no plural forms, so no selector
save-played = { $hours }時間{ $minutes }分プレイ
saves-count = セーブ{ $count }件
```

The glue we write instead of `bevy_fluent` is small, and it keeps the language inside the
ECS as a resource, where change detection can drive the live switch.

New dependencies: `fluent-bundle` and `unic-langid`, plus what they pull (`fluent-syntax`,
`intl-memoizer`, `intl_pluralrules`, `self_cell`, `smallvec`, `rustc-hash`). All are
MIT or Apache-2.0, already in `about.toml`; `just release` checks it. `sys-locale 0.3.2` is
already in `Cargo.lock` (`bevy_text` uses it), so making it direct adds nothing.

### Two Fluent defaults to turn off

1. **Unicode isolation marks.** Fluent wraps every `{ $arg }` in U+2068 and U+2069 so a
   right-to-left name cannot scramble a left-to-right sentence. No game font draws them,
   so they show as boxes around every number. `bundle.set_use_isolating(false)`; RTL is
   out of scope anyway.
2. **The non-`Sync` bundle.** `fluent_bundle::FluentBundle` caches plural rules in a
   `RefCell`, so it cannot live in a Bevy `Resource`. `fluent_bundle::concurrent::FluentBundle`
   is the same API with a `Mutex`-backed cache, and is `Send + Sync`.

## Structure

### `src/i18n.rs`, a shared file

Every screen and overlay shows text, so this is shared code from the first line, at the top
of `src/` like `ui.rs` and `save.rs`.

```rust
/// A language the game can be played in.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) enum Language {
    English,
    Portuguese,
    Japanese,
}
```

| Item | What it is |
| --- | --- |
| `Language::ALL` | the three, in the order the Settings row shows them |
| `Language::code()` | `"en"`, `"pt-BR"`, `"ja"`: the folder under `assets/locales/` and the `LanguageIdentifier` |
| `Language::native_name()` | `"English"`, `"Português"`, `"日本語"`: never translated, so a player lost in a language they cannot read still finds their own |
| `Translations` | a `Resource` with one `concurrent::FluentBundle` per language, built once at `Startup` |
| `Translations::text(language, key)` | the string, or English's, or the key itself plus a `warn!` |
| `Translations::text_with(language, key, &FluentArgs)` | the same, with `$arguments` |
| `Localized(&'static str)` | a component: "this `Text` shows this key, in the current language" |
| `translate_texts` | the system that writes `Text` from `Localized` |

`Language` is the resource that *is* the current language. It is the only copy: the
Settings row is painted from it, `settings.ron` is written from it.

### Why embedded

The three `.ftl` files are read with `include_str!` into the binary, not loaded as assets:

- The text exists from the first frame. With the asset server, every screen would need a
  "wait until the translations arrive" step, and the splash already shows a frame before
  any asset is in.
- A missing or broken file is a failed test and a failed build, not a game shipped with keys
  where words should be.
- The cost is losing hot reload for the `.ftl` files: an edit needs a rebuild. For three
  hand-edited files that rebuild is seconds.

They still live under `assets/locales/` so they ship next to the game in `dist/`, where a
player or modder can read them, and so the folder name says what they are.

### Static text: the `Localized` component

Today a label is spawned with its words:

```rust
(button("New Game"), MenuAction::NewGame)
```

After this design it is spawned with a key:

```rust
(button("menu-new-game"), MenuAction::NewGame)
```

`button(key)` puts `Localized(key)` on the label's `Text` and leaves the `Text` empty.
`translate_texts` fills it:

- every `Text` with `Added<Localized>`: the frame something spawns, and
- every `Localized` text when `Language` has changed: the frame the player picks another.

It runs in `PostUpdate`, before Bevy lays out text, so a new label never shows a frame empty.

That is what makes the switch live: the menu (or the pause menu) under the Settings overlay
is still spawned, its labels carry `Localized`, so they change with the overlay's own text.

`Text::new("Paused")` becomes `(Text::default(), Localized("pause-title"))`, the same way.
The `const` strings (`QUIT_QUESTION`, `OVERWRITE_QUESTION`, ...) become keys:
`const QUIT_QUESTION: &str = "dialog-quit-without-saving"`, and `dialog(question_key, ...)`
puts `Localized` on its question.

### Dynamic text: formatted where it is spawned

Some text is built from values: a save's label, its age, the save status line. These are
formatted once, with `Translations::text_with`, when they are spawned:

| Text | Today | After |
| --- | --- | --- |
| A save's row | `format!("{name} · {hours}h {minutes:02}m played · {} ago", ...)` in `save_label` | `save-label = { $name } · { $played } · { $age }`, built from three keys |
| Its age | `age_label`: `"3d"` | `age-days = { $count }d` (en), `{ $count }日前` (ja) |
| An unnamed save | `"Unnamed"` | `save-unnamed` |
| An autosave | the name `"Autosave"` written inside the file | `save-autosave`, chosen by `is_autosave(path)` when the label is built, so an autosave written in English reads in Japanese too; `AUTOSAVE_NAME` is no longer written |
| The status line | `SAVED_TEXT`, `SAVE_FAILED_TEXT` | `status-saved`, `status-save-failed` |

None of these can be on screen while the language changes: the save lists and the Settings
overlay are both overlays, and only one overlay is open at a time. The status line under
the pause menu can, for its 3.5 s; it keeps the language it was written in, which is not
worth a mechanism.

`save_label` and `age_label` keep their tests; they take `&Translations` and a `Language`,
and each test runs against English and Japanese.

### What is never translated

- **Save names** the player typed. Their own words.
- **The credits' names** ("Heryk13"). The credits text becomes
  `credits-made-with = Made with Bevy` plus the names as plain text.
- **Language names** in the Settings row, as above.
- **Log messages** (`error!`, `warn!`). They are for the developer and stay English.

## The font

Bevy's default font is `FiraMono-subset.ttf`, compiled into `bevy_text`
(`bevy_text-0.19.1/src/lib.rs:82`). It is 18 KB, against roughly ten times that for the
whole of Fira Mono, so it holds little beyond basic Latin: every Japanese character would
draw as an empty box, and Portuguese's `ã`, `ç`, `ê` are at risk too (step 4's *Try* checks
them by typing a save name).

Bevy 0.19 renders text through Parley, and `FontSource::Handle` documents that the default
handle can be replaced: a `Font` inserted at `AssetId::default()` becomes the font of every
`TextFont` that names none, which is every text in this project.

**Proposed: Noto Sans JP**, inserted at the default handle at `Startup`, from
`assets/fonts/NotoSansJP-Regular.ttf`.

| | |
| --- | --- |
| Covers | Latin with Portuguese accents, hiragana, katakana, the common kanji |
| License | SIL Open Font License 1.1: free to ship and embed; `OFL.txt` ships next to it, and `cp -r assets dist/` already copies both |
| Size | a few MB, the largest file in the game after the intro frames |
| Look | the UI changes from a monospace font to a sans-serif one, in every language |

One font for all three languages, rather than a font per language, because a save name can
mix scripts (a Brazilian player naming a save "東京"), and one font draws both.

The rejected options:

- **The OS's fonts** (`bevy_text`'s `system_font_discovery`). Parley would find a Japanese
  font on Windows and macOS, but a Linux machine without CJK fonts installed shows boxes.
- **Subsetting Noto Sans JP** to the characters the `.ftl` files use. Much smaller, but a
  player's save name in kanji the files never used would show boxes.

## The Settings row

```
  Language   ( ) English   (•) Português   ( ) 日本語
```

`bevy_ui_widgets` already has a headless `RadioGroup` and `RadioButton`
(`bevy_ui_widgets-0.19.1/src/radio.rs`): the group emits `ValueChange<Entity>` with the
clicked button, and leaves `Checked` to the app. That is the same shape as the fullscreen
checkbox today:

- each button carries `LanguageOption(Language)`;
- an observer on the group's `ValueChange<Entity>` reads that component and sets the
  `Language` resource;
- an `update_language_radios` system, gated on `Overlay::Settings` like
  `update_fullscreen_checkbox`, puts `Checked` on the button that matches the resource and
  paints it.

The row label "Language" is translated like the rest. Changing it changes the overlay's own
labels at once, which is how the player sees it worked.

## Remembering it

`SettingsFile` gains a field:

```rust
#[serde(default)]
language: Option<Language>,
```

- `Some(language)`: the player chose one.
- `None`: a file from before this design, or no choice yet. The game follows the OS.

`#[serde(default)]` keeps every existing `settings.ron` loading, as `GameData` did for saves,
so `SETTINGS_VERSION` is not raised. `save_settings` already runs on `OnExit(Overlay::Settings)`
and writes the `Language` resource with the rest.

On first run, `load_settings` inserts the language from the OS:

```rust
/// Returns the language for an OS locale such as "pt-BR", "ja-JP" or "en-US": the one whose
/// code it starts with, or English.
fn language_from_locale(locale: &str) -> Language
```

`pt-PT` gets Brazilian Portuguese, the closest of the three. `sys_locale::get_locale()`
returning `None` gets English.

## Files

| File | Change |
| --- | --- |
| `Cargo.toml` | `fluent-bundle`, `unic-langid`, `sys-locale` |
| `src/main.rs` | `mod i18n;`, its plugin before the screens |
| `src/i18n.rs` | new, see "Structure" |
| `assets/locales/en/main.ftl` | new: every string in the game today |
| `assets/locales/pt-BR/main.ftl` | new |
| `assets/locales/ja/main.ftl` | new |
| `assets/fonts/NotoSansJP-Regular.ttf`, `OFL.txt` | new |
| `src/ui.rs` | `button(key)` and `dialog(key, ...)` carry `Localized` |
| `src/screens/menu.rs`, `pause.rs` | keys instead of words |
| `src/overlays/credits.rs`, `settings.rs`, `saves.rs` | keys; Settings gains the row; `save_label` and `age_label` translate |
| `src/overlays/settings.rs` | `SettingsFile::language`, `language_from_locale` |
| `src/save.rs` | `AUTOSAVE_NAME` goes; an autosave's label comes from its path |
| unchanged | `splash.rs`, `loading.rs`, `intro.rs` (no text), `fade.rs`, `assets.rs` |

## Tests

| Function | Where | Guarantees |
| --- | --- | --- |
| `language_from_locale` | `i18n.rs` | `pt-BR`, `pt-PT`, `ja-JP`, `en-US` each find theirs; `fr-FR` and `""` fall back to English |
| every `.ftl` file | `i18n.rs` | parses with no errors (a typo fails the test, not the game) |
| the keys | `i18n.rs` | Portuguese and Japanese have exactly English's keys: a key added to one file and forgotten in another fails here |
| `Translations::text` | `i18n.rs` | a missing key falls back to English, then to the key itself |
| `text_with` | `i18n.rs` | no isolation marks (U+2068, U+2069) in a formatted string |
| `save_label`, `age_label` | `saves.rs` | the existing cases, in English and in Japanese |
| `SettingsFile` | `settings.rs` | a file without `language` still parses |

## Implementation order

One commit each, each passing `just check`, each with a *Try*:

1. **Fluent wired in.** Dependencies, `src/i18n.rs` with `Language`, `Translations`,
   `Localized`, `translate_texts`, and an English `main.ftl` holding one key. One label (the
   menu's New Game) switched to it.
   *Try:* the menu looks exactly as before.
2. **Every static text.** All of English into `main.ftl`; `button`, `dialog`, the menu, the
   pause menu and the overlays switched to keys.
   *Try:* every screen and overlay reads as before; no key shows anywhere.
3. **Every dynamic text.** `save_label`, `age_label`, the status line, Unnamed, the autosave
   label.
   *Try:* the save lists read as before.
4. **The font.** Noto Sans JP at the default handle.
   *Try:* every screen in the new font, nothing cut off or overflowing its button; a save
   named "ação 東京" lists with every character drawn.
5. **Portuguese and Japanese.** The two files, and the test that their keys match English's.
   Temporarily start in Japanese to look at it (not committed).
   *Try:* every screen in Japanese, no boxes, no keys, no text past a button's edge.
6. **The Settings row.** The radio group, live switching.
   *Try:* switch to Japanese from the menu's Settings: the overlay and the menu behind it
   change at once; the same from the pause menu.
7. **Remembered, and the OS default.** `SettingsFile::language`, `language_from_locale`.
   *Try:* choose Portuguese, quit, start: Portuguese. Delete `settings.ron`: the OS language.
8. **Docs.** `README.md` features, `docs/flow.md` (the new shared file, the Settings row),
   `docs/template.md` (where the translations live, how to add a language).

## Answered questions

Put to the user before step 1 and answered on 2026-10-02:

1. **The library.** `fluent-bundle` with our own glue, over `bevy_fluent` and `rust-i18n`.
2. **The font.** Noto Sans JP for everything; the UI stops being monospace.
3. **The selector.** Three radio buttons in a row, over one button that cycles.
4. **The Japanese text.** No native reviewer yet: `ja/main.ftl` ships as a draft, and a
   native speaker should read it before a release.

## What changed while building it

The user asked for every step at once, left uncommitted, so the eight steps are one diff.

1. **The font is an `.otf`, compiled in.** `NotoSansJP-Regular.otf` from `notofonts/noto-cjk`
   (`Sans/SubsetOTF/JP`), the static Regular face (4.5 MB), with its `OFL.txt`. Google Fonts
   only ships Noto Sans JP as a variable `.ttf`. It is `include_bytes!` into the game and put at
   the default handle by `use_ui_font` at `Startup`, not loaded through the asset server: loaded,
   the first frames would draw in Fira Mono, the same reason the `.ftl` files are compiled in.
2. **`Localized` requires `Text`.** `#[require(Text)]`, so `Localized("pause-title")` alone is a
   text; no `(Text::default(), Localized(...))` pair.
3. **`text_button(text)` beside `button(key)`.** A save's row is built from values, so it cannot
   be a key; both go through a private `labeled_button(label)`.
4. **`Translations` is built by `Default`**, through `init_resource`: a `new()` with no
   arguments and no `Default` is a clippy warning.
5. **`language_from_locale` lives in `i18n.rs`**, in an "OS Language" section, since
   `settings.rs` is only its caller. It compares the language part exactly (`pt` with `pt`), not
   a prefix, and splits on `_` too, so `pt_BR` works.
6. **No test of the fallback to English.** Every file has exactly English's keys, a test
   enforces it, so no real key reaches the fallback. The fallback to the key itself is tested.
7. **"ago" moved into the age keys.** `age-days = {  }d ago`, since Portuguese puts "há"
   before the number and Japanese "前" after it. `` is passed already padded (`"01"`).
8. **Credits are two texts**, the translated line and the names, so the overlay's row gap now sits
   between them where a line break was.
9. **`settings.ron` pins the language once Settings closes.** `save_settings` always writes
   `Some(language)`, so a player who opens Settings and changes nothing keeps the OS's language
   from then on, even if the OS changes. Accepted: it is what they saw on the panel.
10. **`fluent-syntax` is a dev-dependency**, for the test that reads each file's keys; it was
    already in the tree through `fluent-bundle`.
11. **Left in English:** the `AccessibleLabel`s on the fullscreen checkbox and the volume slider,
    which a screen reader speaks but nothing draws.
