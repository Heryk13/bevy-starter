use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::path::{Path, PathBuf};

use super::Explorer;
use super::listing::{Kind, date_text, kind_of, read_entry, size_text};
use crate::ui::{ROW_CONTENT_RADIUS_PX, SMALL_FONT_SIZE_PX, TEXT, custom_icon, font, muted_text};

// ==== Config =================================================================

// ---- layout -----------------------------------------------------------------
const PREVIEW_WIDTH_PX: f32 = 300.0;
const PREVIEW_GAP_PX: f32 = 6.0;
// the shape of the frame until a picture gives it its own, the shape of most screens
const FRAME_ASPECT_RATIO: f32 = 16.0 / 9.0;
const FRAME_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.25);
// what stands in for a file that is not a picture
const KIND_ICON_PX: f32 = 72.0;

// ==== Plugin =================================================================

/// The explorer's preview: the selected file, a picture shown large, anything else its icon,
/// with its name, size and date under it.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (show_preview, show_decoded_picture)
            .chain()
            .run_if(resource_exists::<Explorer>),
    );
}

// ==== Widgets ================================================================

/// Returns the preview's column, which `show_preview` fills.
pub(super) fn preview() -> impl Bundle {
    (
        Node {
            width: px(PREVIEW_WIDTH_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(PREVIEW_GAP_PX),
            ..default()
        },
        ExplorerPreview,
    )
}

// ==== Components =============================================================

/// Marks the node the preview is spawned in.
#[derive(Component)]
struct ExplorerPreview;

/// Marks the frame a picture is shown in, once decoded.
#[derive(Component)]
struct PreviewFrame;

/// Marks the line that says a picture's size in pixels, written once it is decoded.
#[derive(Component)]
struct PreviewPixels;

/// A picture being read and decoded on another thread, for the preview. Dropped, as the
/// selection moves on, it is cancelled.
#[derive(Component)]
struct PreviewDecode(Task<Option<Image>>);

// ==== Systems ================================================================

/// Fills the preview from the selected entry when the window opens and when the selection
/// changes: a file's name, size and date, under a frame holding its icon, or for a picture,
/// the picture once another thread has decoded it. A folder, or nothing, leaves it empty.
fn show_preview(
    mut commands: Commands,
    explorer: Res<Explorer>,
    previews: Query<(Entity, Ref<ExplorerPreview>)>,
    decodes: Query<Entity, With<PreviewDecode>>,
    mut shown: Local<Option<Option<PathBuf>>>,
) {
    let Ok((preview, marker)) = previews.single() else {
        *shown = None;
        return;
    };
    let selected = explorer.selected_on_disk();
    if !marker.is_added() && shown.as_ref() == Some(&selected) {
        return;
    }
    commands.entity(preview).despawn_children();
    for decode in &decodes {
        commands.entity(decode).despawn();
    }
    *shown = Some(selected.clone());
    let Some(entry) = selected.as_deref().and_then(read_entry) else {
        return;
    };
    if entry.is_folder {
        return;
    }
    let kind = kind_of(&entry);
    let frame = commands
        .spawn((
            Node {
                width: percent(100),
                aspect_ratio: Some(FRAME_ASPECT_RATIO),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(ROW_CONTENT_RADIUS_PX)),
                ..default()
            },
            BackgroundColor(FRAME_FILL),
            PreviewFrame,
            ChildOf(preview),
        ))
        .id();
    if let (Kind::Image, Some(path)) = (kind, selected) {
        let decode = AsyncComputeTaskPool::get().spawn(async move { decode_picture(&path) });
        commands.spawn(PreviewDecode(decode));
    } else {
        commands.spawn((custom_icon(kind.icon(), KIND_ICON_PX, TEXT), ChildOf(frame)));
    }
    commands.spawn((
        Text::new(entry.name),
        font(SMALL_FONT_SIZE_PX),
        TextColor(TEXT),
        ChildOf(preview),
    ));
    if kind == Kind::Image {
        // written once the picture is decoded and says how large it is
        commands.spawn((
            Text::default(),
            muted_text(SMALL_FONT_SIZE_PX),
            PreviewPixels,
            ChildOf(preview),
        ));
    }
    for line in [size_text(entry.size_bytes), date_text(entry.changed_secs)] {
        commands.spawn((
            Text::new(line),
            muted_text(SMALL_FONT_SIZE_PX),
            ChildOf(preview),
        ));
    }
}

/// Shows the picture a decode finished, in the preview's frame, the frame taking its shape,
/// and writes its size in pixels; or, for a file that would not decode, its icon.
fn show_decoded_picture(
    mut commands: Commands,
    mut decodes: Query<(Entity, &mut PreviewDecode)>,
    mut frames: Query<(Entity, &mut Node), With<PreviewFrame>>,
    mut pixels: Query<&mut Text, With<PreviewPixels>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, mut decode) in &mut decodes {
        let Some(decoded) = check_ready(&mut decode.0) else {
            continue;
        };
        commands.entity(entity).despawn();
        let Ok((frame, mut frame_node)) = frames.single_mut() else {
            continue;
        };
        let Some(image) = decoded else {
            // not a picture this build can read, a compressed texture or a broken file: the
            // frame shows what kind of file it is, as for any other
            commands.spawn((
                custom_icon(Kind::Image.icon(), KIND_ICON_PX, TEXT),
                ChildOf(frame),
            ));
            continue;
        };
        let (width, height) = (image.width(), image.height());
        frame_node.aspect_ratio = Some(width as f32 / height.max(1) as f32);
        commands.spawn((
            ImageNode::new(images.add(image)),
            Node {
                width: percent(100),
                height: percent(100),
                border_radius: BorderRadius::all(px(ROW_CONTENT_RADIUS_PX)),
                ..default()
            },
            ChildOf(frame),
        ));
        for mut text in &mut pixels {
            text.0 = format!("{width} × {height}");
        }
    }
}

// ==== Helpers ================================================================

/// Returns the picture in the file at `path`, read and decoded by its extension, or `None`
/// when it cannot be read as one. Only the renderer keeps it: nothing reads its pixels once
/// it is on screen.
fn decode_picture(path: &Path) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    Image::from_buffer(
        &bytes,
        ImageType::Extension(&extension),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::RENDER_WORLD,
    )
    .ok()
}
