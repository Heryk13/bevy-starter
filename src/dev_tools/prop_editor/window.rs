use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Activate, ScrollArea, TextInput};

use super::PropEditor;
use super::library::{LibraryEntry, PropLibrary, PropPreferences, PropThumbnails};
use crate::ui::{
    ACCENT, BORDER, BORDER_PX, ButtonKind, FIELD_FILL, Icon, SMALL_FONT_SIZE_PX, TEXT, TEXT_MUTED,
    custom_icon, font, laid_out_button, muted_text, words_button,
};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const WINDOW_TITLE: &str = "Props";
const ALL_WORDS: &str = "All";
const FAVOURITES_WORDS: &str = "Favourites";
const RECENT_WORDS: &str = "Recent";
const BY_NAME_WORDS: &str = "Name";
const BY_SIZE_WORDS: &str = "Size";
const EMPTY_WORDS: &str = "No props: drop a file on the window, or put one in assets/props/";

// ---- toolbar: the title, the tabs, the search field and the sort ------------
const TOOLBAR_GAP_PX: f32 = 8.0;
const SEARCH_WIDTH_PX: f32 = 220.0;
const SEARCH_PADDING_X_PX: f32 = 12.0;
const SEARCH_PADDING_Y_PX: f32 = 4.0;
const SEARCH_MAX_CHARS: usize = 40;

// ---- grid: a Finder icon view, a cell a prop --------------------------------
const GRID_GAP_PX: f32 = 10.0;
const CELL_WIDTH_PX: f32 = 112.0;
const CELL_PADDING_PX: f32 = 6.0;
const CELL_RADIUS_PX: f32 = 12.0;
const THUMBNAIL_PX: f32 = 88.0;
const THUMBNAIL_ICON_PX: f32 = 40.0;
const STAR_PX: f32 = 22.0;
const STAR_ICON_PX: f32 = 14.0;
// a star not marked: faint, so the marked ones stand out
const STAR_UNMARKED: Color = Color::srgba(1.0, 1.0, 1.0, 0.3);

// ==== Plugin =================================================================

/// The props window's insides: the tabs (All, Favourites, Recent), the search field, the sort
/// by name or by size, and the grid of thumbnails, each with its name and a star.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PropsView>();
    app.add_systems(
        Update,
        (read_search, (fill_props_controls, fill_props_grid))
            .chain()
            .run_if(in_state(PropEditor::On)),
    );
    app.add_observer(press_tab.run_if(in_state(PropEditor::On)));
    app.add_observer(press_sort.run_if(in_state(PropEditor::On)));
    app.add_observer(press_star.run_if(in_state(PropEditor::On)));
}

// ==== Widgets ================================================================

/// Returns what the props window holds: the toolbar over the grid.
pub(super) fn window_body() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(TOOLBAR_GAP_PX),
            flex_grow: 1.0,
            min_height: px(0),
            ..default()
        },
        children![
            (
                Node {
                    column_gap: px(TOOLBAR_GAP_PX),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new(WINDOW_TITLE),
                        font(SMALL_FONT_SIZE_PX),
                        TextColor(TEXT)
                    ),
                    (
                        Node {
                            column_gap: px(TOOLBAR_GAP_PX),
                            flex_grow: 1.0,
                            ..default()
                        },
                        PropsControls,
                    ),
                    search_field(),
                ],
            ),
            (
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_wrap: FlexWrap::Wrap,
                    align_content: AlignContent::FlexStart,
                    row_gap: px(GRID_GAP_PX),
                    column_gap: px(GRID_GAP_PX),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition::default(),
                ScrollArea,
                PropsGrid,
            ),
        ],
    )
}

/// Returns the search field, which `read_search` reads as it is typed in.
fn search_field() -> impl Bundle {
    (
        EditableText {
            max_characters: Some(SEARCH_MAX_CHARS),
            ..default()
        },
        // `EditableText` only holds the text, this is what types into it
        TextInput,
        // a click only hands the keyboard to an entity with a `TabIndex`
        TabIndex::default(),
        PropsSearch,
        TextCursorStyle {
            color: ACCENT,
            ..default()
        },
        font(SMALL_FONT_SIZE_PX),
        TextColor(TEXT),
        Node {
            width: px(SEARCH_WIDTH_PX),
            padding: UiRect::axes(px(SEARCH_PADDING_X_PX), px(SEARCH_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(FIELD_FILL),
        BorderColor::all(BORDER),
    )
}

// ==== Components =============================================================

/// Marks the row the tabs and the sort button are spawned in.
#[derive(Component)]
struct PropsControls;

/// Marks the grid the props' cells are spawned in.
#[derive(Component)]
struct PropsGrid;

/// Marks the search field.
#[derive(Component)]
struct PropsSearch;

/// A tab's button, with the tab it shows.
#[derive(Component)]
struct TabButton(PropsTab);

/// Marks the button that changes how the grid is sorted.
#[derive(Component)]
struct SortButton;

/// Marks a prop's cell.
#[derive(Component)]
struct PropCell;

/// A prop's star, with its asset path: pressing it marks or unmarks it as a favourite.
#[derive(Component)]
struct PropStar(String);

// ==== Resources ==============================================================

/// What the grid shows: a tab, the text searched for and the order. Kept while the game runs,
/// so the window opens as it was left.
#[derive(Resource, Default, PartialEq)]
struct PropsView {
    tab: PropsTab,
    search: String,
    sort: PropsSort,
}

/// Which props the grid shows: every one, the favourites, or the last ones placed.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum PropsTab {
    #[default]
    All,
    Favourites,
    Recent,
}

/// How the grid is sorted: by name, or the largest file first. Recent keeps its own order.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum PropsSort {
    #[default]
    Name,
    Size,
}

// ==== Systems ================================================================

/// Keeps what is typed in the search field in the view, writing only when it changes.
fn read_search(fields: Query<&EditableText, With<PropsSearch>>, mut view: ResMut<PropsView>) {
    let Ok(field) = fields.single() else {
        return;
    };
    let typed = field.value().to_string();
    if view.search != typed {
        view.search = typed;
    }
}

/// Spawns the tabs and the sort button again whenever the view changes, the tab shown as the
/// primary button, and as the window opens.
fn fill_props_controls(
    mut commands: Commands,
    view: Res<PropsView>,
    rows: Query<(Entity, Ref<PropsControls>)>,
) {
    let Ok((row, added)) = rows.single() else {
        return;
    };
    if !view.is_changed() && !added.is_added() {
        return;
    }
    commands.entity(row).despawn_children();
    for (tab, words) in [
        (PropsTab::All, ALL_WORDS),
        (PropsTab::Favourites, FAVOURITES_WORDS),
        (PropsTab::Recent, RECENT_WORDS),
    ] {
        let kind = if tab == view.tab {
            ButtonKind::Primary
        } else {
            ButtonKind::Choice
        };
        commands.spawn((words_button(words, kind), TabButton(tab), ChildOf(row)));
    }
    let sort_words = match view.sort {
        PropsSort::Name => BY_NAME_WORDS,
        PropsSort::Size => BY_SIZE_WORDS,
    };
    commands.spawn((
        words_button(sort_words, ButtonKind::Quiet),
        SortButton,
        ChildOf(row),
    ));
}

/// Spawns the grid's cells again whenever what it shows changes: the library, the view, the
/// favourites and recents, a thumbnail made; and as the window opens.
fn fill_props_grid(
    mut commands: Commands,
    library: Res<PropLibrary>,
    view: Res<PropsView>,
    preferences: Res<PropPreferences>,
    thumbnails: Res<PropThumbnails>,
    grids: Query<(Entity, Ref<PropsGrid>)>,
) {
    let Ok((grid, added)) = grids.single() else {
        return;
    };
    let changed = library.is_changed()
        || view.is_changed()
        || preferences.is_changed()
        || thumbnails.is_changed();
    if !changed && !added.is_added() {
        return;
    }
    commands.entity(grid).despawn_children();
    let shown = shown_props(&library.0, &view, &preferences);
    if shown.is_empty() && view.tab == PropsTab::All && view.search.is_empty() {
        commands.spawn((
            Text::new(EMPTY_WORDS),
            muted_text(SMALL_FONT_SIZE_PX),
            ChildOf(grid),
        ));
    }
    for entry in shown {
        let thumbnail = thumbnails.0.get(&entry.file).cloned();
        let favourite = preferences.favourites.contains(&entry.file);
        commands.spawn((prop_cell(entry, thumbnail, favourite), ChildOf(grid)));
    }
}

/// Shows the tab pressed.
fn press_tab(activate: On<Activate>, tabs: Query<&TabButton>, mut view: ResMut<PropsView>) {
    if let Ok(tab) = tabs.get(activate.entity) {
        view.tab = tab.0;
    }
}

/// Sorts the other way when the sort button is pressed: by name, then by size.
fn press_sort(
    activate: On<Activate>,
    buttons: Query<(), With<SortButton>>,
    mut view: ResMut<PropsView>,
) {
    if buttons.contains(activate.entity) {
        view.sort = match view.sort {
            PropsSort::Name => PropsSort::Size,
            PropsSort::Size => PropsSort::Name,
        };
    }
}

/// Marks or unmarks a favourite when its star is pressed.
fn press_star(
    activate: On<Activate>,
    stars: Query<&PropStar>,
    mut preferences: ResMut<PropPreferences>,
) {
    if let Ok(star) = stars.get(activate.entity) {
        preferences.toggle_favourite(&star.0);
    }
}

// ==== Helpers ================================================================

/// Returns a prop's cell: its thumbnail, or an icon while it has none, and its name, in a
/// button that takes a new one into the hand, with its star under it.
fn prop_cell(
    entry: &LibraryEntry,
    thumbnail: Option<Handle<Image>>,
    favourite: bool,
) -> impl Bundle {
    // one bundle either way, the image or the icon seen-through: a function returns one type
    let (image_color, icon_color) = match thumbnail {
        Some(_) => (Color::WHITE, Color::NONE),
        None => (Color::NONE, TEXT_MUTED),
    };
    let picture = (
        ImageNode {
            image: thumbnail.unwrap_or_default(),
            color: image_color,
            ..default()
        },
        Node {
            width: px(THUMBNAIL_PX),
            height: px(THUMBNAIL_PX),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            custom_icon(Icon::Box, THUMBNAIL_ICON_PX, icon_color),
            Pickable::IGNORE
        )],
    );
    let star_color = if favourite { TEXT } else { STAR_UNMARKED };
    (
        Node {
            width: px(CELL_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                laid_out_button(
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(CELL_PADDING_PX)),
                        border_radius: BorderRadius::all(px(CELL_RADIUS_PX)),
                        ..default()
                    },
                    ButtonKind::Choice,
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![
                            (picture, Pickable::IGNORE),
                            (
                                Text::new(entry.name.clone()),
                                font(SMALL_FONT_SIZE_PX),
                                TextColor(TEXT),
                                Pickable::IGNORE,
                            ),
                        ],
                    ),
                ),
                PropCell,
            ),
            (
                laid_out_button(
                    Node {
                        width: px(STAR_PX),
                        height: px(STAR_PX),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    ButtonKind::Quiet,
                    (
                        custom_icon(Icon::Star, STAR_ICON_PX, star_color),
                        Pickable::IGNORE
                    ),
                ),
                PropStar(entry.file.clone()),
            ),
        ],
    )
}

/// Returns the props the grid shows from `entries`: those of the view's tab whose name holds
/// the text searched for, whatever its case, in the view's order; Recent keeps its own, the
/// newest first.
fn shown_props<'a>(
    entries: &'a [LibraryEntry],
    view: &PropsView,
    preferences: &PropPreferences,
) -> Vec<&'a LibraryEntry> {
    let search = view.search.trim().to_lowercase();
    let matches = |entry: &&LibraryEntry| entry.name.to_lowercase().contains(&search);
    let mut shown: Vec<&LibraryEntry> = match view.tab {
        PropsTab::All => entries.iter().filter(matches).collect(),
        PropsTab::Favourites => entries
            .iter()
            .filter(|entry| preferences.favourites.contains(&entry.file))
            .filter(matches)
            .collect(),
        PropsTab::Recent => {
            return preferences
                .recent
                .iter()
                .filter_map(|file| entries.iter().find(|entry| &entry.file == file))
                .filter(matches)
                .collect();
        }
    };
    match view.sort {
        PropsSort::Name => shown.sort_by_key(|entry| entry.name.to_lowercase()),
        PropsSort::Size => shown.sort_by_key(|entry| std::cmp::Reverse(entry.size)),
    }
    shown
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, size: u64) -> LibraryEntry {
        LibraryEntry {
            file: format!("props/{name}"),
            name: name.to_string(),
            size,
        }
    }

    fn names(shown: &[&LibraryEntry]) -> Vec<String> {
        shown.iter().map(|entry| entry.name.clone()).collect()
    }

    #[test]
    fn search_keeps_the_names_holding_the_text_whatever_their_case() {
        let entries = [
            entry("Pine.glb", 1),
            entry("rock.glb", 1),
            entry("PINECONE.glb", 1),
        ];
        let view = PropsView {
            search: "pine".to_string(),
            ..default()
        };
        let shown = shown_props(&entries, &view, &PropPreferences::default());
        assert_eq!(names(&shown), ["Pine.glb", "PINECONE.glb"]);
    }

    #[test]
    fn sorting_by_size_puts_the_largest_first() {
        let entries = [entry("a.glb", 10), entry("b.glb", 30), entry("c.glb", 20)];
        let view = PropsView {
            sort: PropsSort::Size,
            ..default()
        };
        let shown = shown_props(&entries, &view, &PropPreferences::default());
        assert_eq!(names(&shown), ["b.glb", "c.glb", "a.glb"]);
    }

    #[test]
    fn sorting_by_name_ignores_case() {
        let entries = [entry("b.glb", 1), entry("A.glb", 1), entry("c.glb", 1)];
        let shown = shown_props(&entries, &PropsView::default(), &PropPreferences::default());
        assert_eq!(names(&shown), ["A.glb", "b.glb", "c.glb"]);
    }

    #[test]
    fn favourites_shows_only_the_starred() {
        let entries = [entry("a.glb", 1), entry("b.glb", 1)];
        let preferences = PropPreferences {
            favourites: vec!["props/b.glb".to_string()],
            ..default()
        };
        let view = PropsView {
            tab: PropsTab::Favourites,
            ..default()
        };
        assert_eq!(
            names(&shown_props(&entries, &view, &preferences)),
            ["b.glb"]
        );
    }

    #[test]
    fn recent_keeps_the_order_placed_and_leaves_out_files_gone() {
        let entries = [entry("a.glb", 1), entry("b.glb", 1)];
        let preferences = PropPreferences {
            recent: vec![
                "props/b.glb".to_string(),
                "props/gone.glb".to_string(),
                "props/a.glb".to_string(),
            ],
            ..default()
        };
        let view = PropsView {
            tab: PropsTab::Recent,
            ..default()
        };
        let shown = shown_props(&entries, &view, &preferences);
        assert_eq!(names(&shown), ["b.glb", "a.glb"]);
    }
}
