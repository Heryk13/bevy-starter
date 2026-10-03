use bevy::prelude::*;
use bevy::ui::UiSystems;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

// ==== Config =================================================================

// ---- translations: compiled in, so every word exists from the first frame ---
const ENGLISH_FTL: &str = include_str!("../assets/locales/en/main.ftl");
const PORTUGUESE_FTL: &str = include_str!("../assets/locales/pt-BR/main.ftl");
const JAPANESE_FTL: &str = include_str!("../assets/locales/ja/main.ftl");

// ---- font: one face for every language, Latin accents and Japanese alike ----
// compiled in for the same reason as the translations: loaded as an asset, the first
// frames would draw in Bevy's own font, which has neither
const UI_FONT_DATA: &[u8] = include_bytes!("../assets/fonts/NotoSansJP-Regular.otf");

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Language>();
    app.init_resource::<Translations>();
    app.add_systems(Startup, use_ui_font);
    // before the UI lays out its text, so a new label never shows a frame empty
    app.add_systems(PostUpdate, translate_texts.before(UiSystems::Prepare));
}

// ==== OS Language ============================================================

/// Returns the language an OS locale such as "pt-BR", "ja-JP" or "en-US" asks for: the one
/// whose language part it shares, so "pt-PT" gets Brazilian Portuguese, the closest of the
/// three. Any other locale gets English.
pub(crate) fn language_from_locale(locale: &str) -> Language {
    // either separator: a locale may arrive as "pt_BR" as well as "pt-BR"
    let language_part = locale.split(['-', '_']).next();
    Language::ALL
        .into_iter()
        .find(|language| language.code().split('-').next() == language_part)
        .unwrap_or_default()
}

// ==== Components =============================================================

/// Shows the text of a translation key, in the current language, on its entity's `Text`.
#[derive(Component)]
#[require(Text)]
pub(crate) struct Localized(pub(crate) &'static str);

// ==== Resources ==============================================================

/// The language the game is played in. It is the only copy: the Settings row is painted
/// from it, and `settings.ron` is written from it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub(crate) enum Language {
    #[default]
    English,
    Portuguese,
    Japanese,
}

impl Language {
    /// Every language, in the order the Settings row shows them.
    pub(crate) const ALL: [Language; 3] = [Self::English, Self::Portuguese, Self::Japanese];

    /// Returns the language's code: its folder under `assets/locales/`.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Portuguese => "pt-BR",
            Self::Japanese => "ja",
        }
    }

    /// Returns the language's name in itself. Never translated, so a player lost in a
    /// language they cannot read still finds their own.
    pub(crate) fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
            Self::Japanese => "日本語",
        }
    }

    /// Returns the text of the language's translation file.
    fn source(self) -> &'static str {
        match self {
            Self::English => ENGLISH_FTL,
            Self::Portuguese => PORTUGUESE_FTL,
            Self::Japanese => JAPANESE_FTL,
        }
    }
}

/// Every language's translations, built once at startup.
#[derive(Resource)]
pub(crate) struct Translations {
    bundles: [FluentBundle<FluentResource>; Language::ALL.len()],
}

impl Default for Translations {
    /// Builds the translations from the files compiled into the game. A file with a typo
    /// keeps every message that parsed, and logs the rest.
    fn default() -> Self {
        Self {
            bundles: Language::ALL.map(bundle),
        }
    }
}

impl Translations {
    /// Returns the text of `key` in `language`.
    pub(crate) fn text(&self, language: Language, key: &str) -> String {
        self.format(language, key, None)
    }

    /// Returns the text of `key` in `language`, with its `$arguments` filled in.
    pub(crate) fn text_with(&self, language: Language, key: &str, args: &FluentArgs) -> String {
        self.format(language, key, Some(args))
    }

    /// Returns the text of `key` in `language`, or in English when that language lacks it,
    /// or the key itself, so a missing word shows where it is missing instead of a blank.
    fn format(&self, language: Language, key: &str, args: Option<&FluentArgs>) -> String {
        [language, Language::English]
            .into_iter()
            .find_map(|language| {
                // the array is built from `Language::ALL`, so every language has its bundle
                let bundle = &self.bundles[language as usize];
                let pattern = bundle.get_message(key)?.value()?;
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, args, &mut errors);
                if !errors.is_empty() {
                    warn!("translating {key} in {}: {errors:?}", language.code());
                }
                Some(text.into_owned())
            })
            .unwrap_or_else(|| {
                warn!("no translation for {key}");
                key.to_string()
            })
    }
}

// ==== Systems ================================================================

/// Replaces Bevy's default font, a small subset of Fira Mono with no Japanese and few
/// accents, with Noto Sans JP. Every text that names no font of its own uses this one.
fn use_ui_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(UI_FONT_DATA.to_vec());
    if let Err(error) = fonts.insert(AssetId::default(), font) {
        error!("could not replace the default font: {error}");
    }
}

/// Writes every `Localized` text the frame it spawns, and all of them again the frame the
/// language changes, which is what switches the menu behind the Settings panel too.
fn translate_texts(
    language: Res<Language>,
    translations: Res<Translations>,
    mut texts: Query<(&mut Text, Ref<Localized>)>,
) {
    for (mut text, localized) in &mut texts {
        if language.is_changed() || localized.is_added() {
            text.0 = translations.text(*language, localized.0);
        }
    }
}

// ==== Helpers ================================================================

/// Returns the bundle that translates into `language`.
fn bundle(language: Language) -> FluentBundle<FluentResource> {
    // the codes are written above, so a parse error would be a typo caught by the tests;
    // the default ("und") would only lose the language's plural rules
    let id: LanguageIdentifier = language.code().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // no Unicode isolation marks around arguments: no font draws them, and right-to-left
    // text, which they exist for, is out of scope
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(language.source().to_string()).unwrap_or_else(
        |(resource, errors)| {
            error!("{} translations have errors: {errors:?}", language.code());
            resource
        },
    );
    if let Err(errors) = bundle.add_resource(resource) {
        error!("{} translations repeat keys: {errors:?}", language.code());
    }
    bundle
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
    use std::collections::{BTreeMap, BTreeSet};

    /// Returns the message keys a translation file defines.
    fn keys(source: &str) -> BTreeSet<String> {
        let resource = FluentResource::try_new(source.to_string()).unwrap();
        resource
            .entries()
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Returns the variables each message of a translation file uses in its value, by key.
    // ponytail: message attributes and function arguments (`NUMBER($x)`) are not walked, as
    // no .ftl here uses either; walk them the day one does
    fn variables(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let resource = FluentResource::try_new(source.to_string()).unwrap();
        resource
            .entries()
            .filter_map(|entry| match entry {
                Entry::Message(message) => {
                    let mut names = BTreeSet::new();
                    if let Some(pattern) = &message.value {
                        pattern_variables(pattern, &mut names);
                    }
                    Some((message.id.name.to_string(), names))
                }
                _ => None,
            })
            .collect()
    }

    /// Adds the variables a pattern uses, including inside each variant of a select.
    fn pattern_variables(pattern: &Pattern<&str>, names: &mut BTreeSet<String>) {
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                expression_variables(expression, names);
            }
        }
    }

    /// Adds the variables an expression uses: its own, a select's selector, and its variants'.
    fn expression_variables(expression: &Expression<&str>, names: &mut BTreeSet<String>) {
        let inline = match expression {
            Expression::Inline(inline) => inline,
            Expression::Select { selector, variants } => {
                for variant in variants {
                    pattern_variables(&variant.value, names);
                }
                selector
            }
        };
        match inline {
            InlineExpression::VariableReference { id } => {
                names.insert(id.name.to_string());
            }
            InlineExpression::Placeable { expression } => expression_variables(expression, names),
            _ => {}
        }
    }

    // ---- language_from_locale -----------------------------------------------

    #[test]
    fn brazilian_locale_gets_portuguese() {
        assert_eq!(language_from_locale("pt-BR"), Language::Portuguese);
    }

    #[test]
    fn european_portuguese_locale_gets_brazilian_portuguese() {
        assert_eq!(language_from_locale("pt-PT"), Language::Portuguese);
    }

    #[test]
    fn japanese_locale_gets_japanese() {
        assert_eq!(language_from_locale("ja-JP"), Language::Japanese);
    }

    #[test]
    fn american_locale_gets_english() {
        assert_eq!(language_from_locale("en-US"), Language::English);
    }

    #[test]
    fn other_locale_falls_back_to_english() {
        assert_eq!(language_from_locale("fr-FR"), Language::English);
    }

    #[test]
    fn underscore_locale_is_read_like_a_dash() {
        assert_eq!(language_from_locale("pt_BR"), Language::Portuguese);
    }

    #[test]
    fn empty_locale_falls_back_to_english() {
        assert_eq!(language_from_locale(""), Language::English);
    }

    // ---- translation files --------------------------------------------------

    #[test]
    fn every_translation_file_parses() {
        for language in Language::ALL {
            assert!(
                FluentResource::try_new(language.source().to_string()).is_ok(),
                "{} has a syntax error",
                language.code()
            );
        }
    }

    #[test]
    fn every_language_has_exactly_the_english_keys() {
        let english = keys(ENGLISH_FTL);
        for language in Language::ALL {
            assert_eq!(keys(language.source()), english, "{}", language.code());
        }
    }

    #[test]
    fn no_translation_uses_a_variable_english_lacks() {
        let english = variables(ENGLISH_FTL);
        for language in Language::ALL {
            for (key, names) in variables(language.source()) {
                let known = english.get(&key).cloned().unwrap_or_default();
                let unknown: BTreeSet<_> = names.difference(&known).collect();
                assert!(unknown.is_empty(), "{} {key}: {unknown:?}", language.code());
            }
        }
    }

    #[test]
    fn every_language_code_is_a_valid_identifier() {
        for language in Language::ALL {
            assert!(language.code().parse::<LanguageIdentifier>().is_ok());
        }
    }

    // ---- Translations -------------------------------------------------------

    #[test]
    fn key_is_translated_into_the_language() {
        let translations = Translations::default();
        assert_eq!(translations.text(Language::Japanese, "dialog-yes"), "はい");
    }

    #[test]
    fn missing_key_shows_the_key_itself() {
        let translations = Translations::default();
        assert_eq!(
            translations.text(Language::Japanese, "no-such-key"),
            "no-such-key"
        );
    }

    #[test]
    fn formatted_text_has_no_isolation_marks() {
        let translations = Translations::default();
        let mut args = FluentArgs::new();
        args.set("count", 3);
        let text = translations.text_with(Language::English, "age-days", &args);
        assert_eq!(text, "3d ago");
    }
}
