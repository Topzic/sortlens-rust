//! The map's own pieces: styles, tile textures, dot sizes, and which dot a click landed on.
//!
//! Kept apart from the view for the same reason [\`crate::sharpness\`] is: the arithmetic that decides *where* a dot
//! goes, *what was clicked* and *what a tile looks like* is worth testing on its own, and a test that has to build a
//! window to check a hit test is a test nobody writes twice.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use egui::{Color32, ColorImage, TextureHandle};
use maplibre_egui::EguiMap;
use maplibre_egui::model::{Camera, CameraChange, Command, Content, Inputs, Options, Record};
use sortlens_core::PhotoId;
use sortlens_geo::LatLon;
use sortlens_geo::tiles::TileSet;
use walkers::sources::Attribution;
use walkers::{Tile, TileId, TilePiece, Tiles};

/// The deepest zoom Places offers when no tile set says otherwise.
///
/// The shipped coastline has no detail this deep, but the pins do: zooming in is how a cluster of
/// photographs from one street comes apart, so the map must not stop at the outline's own ceiling.
pub const MAX_ZOOM: u8 = 18;

/// The radius of the smallest dot, in pixels. Small enough that a single photograph is a point rather than a blob.
pub const MIN_DOT: f32 = 4.5;

/// How much each doubling of a dot's photo count adds to its radius.
///
/// Square-root growth, so area tracks the count: a dot standing for four hundred photographs must not be a hundred
/// times the width of one, or it covers the coastline it is meant to be beside.
pub const DOT_GROWTH: f32 = 2.2;

/// The radius a dot stops growing at. Past this it is a region, and a bigger circle stops meaning anything.
pub const MAX_DOT: f32 = 26.0;

/// How many decoded tiles are kept.
///
/// A viewport of 1920×1080 at 256-pixel tiles holds about forty; eight screens' worth is enough to pan back and forth
/// without decoding again, and small enough that a long session does not grow a texture atlas without end.
pub const CACHE_LIMIT: usize = 512;

/// How far a dot may be from a named place and still borrow its name.
///
/// Thirty kilometres is a city and its surroundings: near enough that "Brighton" is the honest answer for a photograph
/// taken on the South Downs, far enough that a village must be the village the photographs are actually in. The
/// gazetteer file decides what exists; this decides how much slack a match is given.
pub const LABEL_KM: f64 = 30.0;

/// A tile larger than this is not a tile. The bound is read before the file is, so a stray archive in the tiles folder
/// cannot be decoded into memory on a frame.
pub const MAX_TILE_BYTES: u64 = 8 * 1024 * 1024;

/// The edge of a tile handed to walkers, in pixels. The set decodes every image at this size.
pub const TILE_PX: u32 = 256;

/// A way of drawing the basemap, independent of the photographs on it.
///
/// These are cartographic treatments, not theme presets: the pins always use the accent colour, and switching the map
/// style never changes the rest of the interface. Auto follows the interface's light/dark decision; the rest are
/// deliberately fixed so a person can choose a paper map in a dark room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MapStyle {
    /// Follow the theme: Original when a tile set is installed, Ink on a dark interface, Paper on a light one.
    #[default]
    Auto,
    /// Leave user-supplied tiles as their author drew them.
    Original,
    /// Charcoal water, graphite land, a quiet shoreline. The default for dark interfaces.
    Ink,
    /// Warm paper land and pale water. The default for light interfaces.
    Paper,
    /// Deep navy and cyan, after a drafting blueprint.
    Blueprint,
    /// Neutral greys with a strong shoreline, for reading the photographs rather than the place.
    Mono,
}

impl MapStyle {
    /// Every style, in menu order.
    pub const ALL: [Self; 6] = [Self::Auto, Self::Original, Self::Ink, Self::Paper, Self::Blueprint, Self::Mono];

    /// The stable identifier, should the choice ever be written to settings.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Original => "original",
            Self::Ink => "ink",
            Self::Paper => "paper",
            Self::Blueprint => "blueprint",
            Self::Mono => "mono",
        }
    }

    /// The English catalogue key for the style's name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Original => "Original",
            Self::Ink => "Ink",
            Self::Paper => "Paper",
            Self::Blueprint => "Blueprint",
            Self::Mono => "Mono",
        }
    }

    /// Resolve Auto into the concrete treatment this frame will draw.
    ///
    /// A tile set is left alone by Auto: the person installed a real map, and tinting it before they asked would be a
    /// surprise. With no tiles, the shipped land and coastline follow the interface's light/dark choice.
    #[must_use]
    pub fn resolve(self, tokens: &crate::theme::Tokens, has_tiles: bool) -> Self {
        if self != Self::Auto {
            return self;
        }
        if has_tiles {
            return Self::Original;
        }
        let luminance = 0.2126 * f32::from(tokens.bg.r()) + 0.7152 * f32::from(tokens.bg.g()) + 0.0722 * f32::from(tokens.bg.b());
        if luminance < 128.0 { Self::Ink } else { Self::Paper }
    }

    /// The colours for the shipped geography and the map's chrome.
    #[must_use]
    pub fn palette(self, tokens: &crate::theme::Tokens) -> MapPalette {
        match self {
            Self::Auto | Self::Original => MapPalette {
                water: tokens.thumb_bg,
                land: tokens.card,
                coast: tokens.map_coast,
                border: tokens.border,
                river: tokens.border.gamma_multiply(0.8),
                graticule: tokens.border.gamma_multiply(0.55),
                frame: tokens.border,
                label: tokens.text_muted,
                credit: tokens.text_faint,
            },
            Self::Ink => MapPalette {
                water: Color32::from_rgb(0x14, 0x18, 0x1a),
                land: Color32::from_rgb(0x21, 0x27, 0x2b),
                coast: Color32::from_rgb(0x45, 0x52, 0x59),
                border: Color32::from_rgb(0x36, 0x40, 0x46),
                river: Color32::from_rgb(0x2e, 0x42, 0x4c),
                graticule: Color32::from_rgb(0x2b, 0x33, 0x38),
                frame: Color32::from_rgb(0x35, 0x3d, 0x42),
                label: Color32::from_rgb(0xc2, 0xca, 0xce),
                credit: Color32::from_rgb(0x78, 0x82, 0x87),
            },
            Self::Paper => MapPalette {
                water: Color32::from_rgb(0xd9, 0xe6, 0xe5),
                land: Color32::from_rgb(0xf4, 0xef, 0xe2),
                coast: Color32::from_rgb(0x8c, 0x85, 0x76),
                border: Color32::from_rgb(0xc3, 0xb9, 0xa5),
                river: Color32::from_rgb(0x9d, 0xbf, 0xc2),
                graticule: Color32::from_rgb(0xda, 0xd2, 0xc1),
                frame: Color32::from_rgb(0xb9, 0xb1, 0xa0),
                label: Color32::from_rgb(0x3c, 0x39, 0x32),
                credit: Color32::from_rgb(0x6f, 0x69, 0x5c),
            },
            Self::Blueprint => MapPalette {
                water: Color32::from_rgb(0x0c, 0x22, 0x3d),
                land: Color32::from_rgb(0x12, 0x33, 0x59),
                coast: Color32::from_rgb(0x58, 0x98, 0xc8),
                border: Color32::from_rgb(0x2c, 0x5f, 0x8b),
                river: Color32::from_rgb(0x3f, 0x84, 0xb5),
                graticule: Color32::from_rgb(0x1c, 0x41, 0x6c),
                frame: Color32::from_rgb(0x24, 0x4c, 0x77),
                label: Color32::from_rgb(0xc0, 0xd9, 0xee),
                credit: Color32::from_rgb(0x79, 0x9d, 0xbc),
            },
            Self::Mono => MapPalette {
                water: Color32::from_rgb(0xe7, 0xe7, 0xe5),
                land: Color32::from_rgb(0xf7, 0xf7, 0xf5),
                coast: Color32::from_rgb(0x6f, 0x6f, 0x6b),
                border: Color32::from_rgb(0xbd, 0xbd, 0xb7),
                river: Color32::from_rgb(0xa5, 0xa5, 0x9f),
                graticule: Color32::from_rgb(0xd0, 0xd0, 0xcc),
                frame: Color32::from_rgb(0xa8, 0xa8, 0xa2),
                label: Color32::from_rgb(0x22, 0x22, 0x20),
                credit: Color32::from_rgb(0x66, 0x66, 0x62),
            },
        }
    }

    /// Restate a decoded raster in this style.
    ///
    /// The conversion is deliberately a duotone rather than an overlay: an overlay hides roads and labels, while a
    /// duotone keeps their contrast and changes only the ink. Original and Auto are a no-op.
    pub fn shade(self, image: &mut ColorImage) {
        if matches!(self, Self::Auto | Self::Original) {
            return;
        }
        for pixel in &mut image.pixels {
            let [r, g, b, a] = pixel.to_array();
            let l = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
            let [nr, ng, nb] = match self {
                Self::Ink => [12.0 + 0.16 * l, 16.0 + 0.18 * l, 18.0 + 0.19 * l],
                Self::Paper => {
                    let v = 240.0 - (255.0 - l) * 0.34;
                    [v + 5.0, v, v - 9.0]
                }
                Self::Blueprint => {
                    let v = l / 255.0;
                    [8.0 + 22.0 * v, 26.0 + 82.0 * v, 48.0 + 142.0 * v]
                }
                Self::Mono => {
                    let v = (128.0 + (l - 128.0) * 1.18).clamp(0.0, 255.0);
                    [v, v, v]
                }
                Self::Auto | Self::Original => [f32::from(r), f32::from(g), f32::from(b)],
            };
            *pixel = Color32::from_rgba_unmultiplied(
                nr.clamp(0.0, 255.0) as u8,
                ng.clamp(0.0, 255.0) as u8,
                nb.clamp(0.0, 255.0) as u8,
                a,
            );
        }
    }
}

/// A cartographic style served by [OpenFreeMap](https://openfreemap.org).
///
/// These are the real MapLibre styles, not recoloured approximations: the map is fetched and drawn only after the
/// person accepts the online map, and the shipped land/coastline map remains the default and the fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpenFreeMapStyle {
    /// OpenFreeMap's general-purpose light style, with land use, roads and labels.
    #[default]
    Liberty,
    /// A brighter general-purpose style.
    Bright,
    /// A quiet light style for reading data over.
    Positron,
    /// A dark style for a dark room.
    Dark,
    /// A muted blue-green style.
    Fiord,
}

impl OpenFreeMapStyle {
    /// Every style, in menu order.
    pub const ALL: [Self; 5] = [Self::Liberty, Self::Bright, Self::Positron, Self::Dark, Self::Fiord];

    /// The style's path on OpenFreeMap, also its stable identifier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Liberty => "liberty",
            Self::Bright => "bright",
            Self::Positron => "positron",
            Self::Dark => "dark",
            Self::Fiord => "fiord",
        }
    }

    /// The English catalogue key for the style's name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Liberty => "Liberty",
            Self::Bright => "Bright",
            Self::Positron => "Positron",
            Self::Dark => "Dark",
            Self::Fiord => "Fiord",
        }
    }

    /// The MapLibre style document's URL.
    pub const fn url(self) -> &'static str {
        match self {
            Self::Liberty => "https://tiles.openfreemap.org/styles/liberty",
            Self::Bright => "https://tiles.openfreemap.org/styles/bright",
            Self::Positron => "https://tiles.openfreemap.org/styles/positron",
            Self::Dark => "https://tiles.openfreemap.org/styles/dark",
            Self::Fiord => "https://tiles.openfreemap.org/styles/fiord",
        }
    }

    /// The style Auto would pick for an interface: Dark in a dark room, Liberty in a light one.
    #[must_use]
    pub fn auto(tokens: &crate::theme::Tokens) -> Self {
        let luminance = 0.2126 * f32::from(tokens.bg.r()) + 0.7152 * f32::from(tokens.bg.g()) + 0.0722 * f32::from(tokens.bg.b());
        if luminance < 128.0 { Self::Dark } else { Self::Liberty }
    }
}

/// The credit OpenFreeMap's data requires, for the map's own footer.
pub const OPENFREEMAP_CREDIT: &str = "OpenFreeMap © OpenMapTiles · Data from OpenStreetMap";

/// The render state eframe owns, kept so Places can hand it to the online map.
///
/// An online map draws through egui-wgpu, which is only available once the window has a wgpu renderer. Tests and the
/// headless screenshot harness do not, so this stays None and Places keeps the offline map.
#[derive(Clone, Default)]
pub struct MapRenderState(Option<Arc<egui_wgpu::RenderState>>);

impl fmt::Debug for MapRenderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapRenderState").field("present", &self.0.is_some()).finish()
    }
}

impl MapRenderState {
    /// Record the window's renderer, or clear it when the window has none.
    pub fn set(&mut self, render_state: Option<&egui_wgpu::RenderState>) {
        match render_state {
            Some(render_state) => {
                if self.0.is_none() {
                    self.0 = Some(Arc::new(render_state.clone()));
                }
            }
            None => self.0 = None,
        }
    }

    /// The renderer, cloned out so a map can be drawn without borrowing the shell's whole state.
    #[must_use]
    pub fn get(&self) -> Option<Arc<egui_wgpu::RenderState>> {
        self.0.clone()
    }
}

/// A live online map, and what the shell knows of it after its last frame.
///
/// Kept behind Rc<RefCell<..>> because EguiMap is not Clone or Debug, while UiState is both. The map owns its
/// texture and its gestures; the shell owns the pins and reads the record after show.
pub struct OnlineMap {
    map: EguiMap,
    style: OpenFreeMapStyle,
    record: Option<Record>,
    error: Option<String>,
}

impl fmt::Debug for OnlineMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnlineMap")
            .field("style", &self.style)
            .field("loaded", &self.loaded())
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl OnlineMap {
    /// A map of OpenFreeMap's style, centred on centre at zoom.
    #[must_use]
    pub fn new(style: OpenFreeMapStyle, centre: LatLon, zoom: f64) -> Self {
        let inputs = Inputs {
            style: style.url().to_owned(),
            camera: Camera {
                center: [centre.lon, centre.lat],
                zoom,
                bearing: 0.0,
                pitch: 0.0,
            },
            content: Content::default(),
            options: Options {
                min_zoom: Some(1.0),
                max_zoom: Some(f64::from(MAX_ZOOM)),
                // Places is a two-dimensional map for now: a pin's geographic point must be the point a click
                // reaches, and a turned or pitched map would need a second pin geometry to keep that true.
                drag_rotate: Some(false),
                pitch_with_rotate: Some(false),
                touch_pitch: Some(false),
                ..Options::default()
            },
            events: Vec::new(),
        };
        Self {
            map: EguiMap::new(inputs),
            style,
            record: None,
            error: None,
        }
    }

    /// The style the map is drawing.
    #[must_use]
    pub fn style(&self) -> OpenFreeMapStyle {
        self.style
    }

    /// Replace the style, keeping the camera.
    pub fn set_style(&mut self, style: OpenFreeMapStyle) {
        if style == self.style {
            return;
        }
        self.style = style;
        let mut inputs = self.map.inputs().clone();
        inputs.style = style.url().to_owned();
        self.map.set_inputs(inputs);
        self.record = None;
        self.error = None;
    }

    /// Whether the style and every tile in view have loaded.
    #[must_use]
    pub fn loaded(&self) -> bool {
        self.record.as_ref().is_some_and(|record| record.loaded)
    }

    /// The camera where the map last came to rest.
    #[must_use]
    pub fn camera(&self) -> Option<Camera> {
        self.record.as_ref().map(|record| record.camera)
    }

    /// The last failure, from the style or a tile, while there is one.
    #[must_use]
    pub fn failure(&self) -> Option<&str> {
        self.error
            .as_deref()
            .or_else(|| self.record.as_ref().and_then(|record| record.failure.as_deref()))
    }

    /// Who the data comes from, as the style declares it.
    #[must_use]
    pub fn attribution(&self) -> &str {
        self.record
            .as_ref()
            .map(|record| record.attribution.as_str())
            .filter(|attribution| !attribution.is_empty())
            .unwrap_or(OPENFREEMAP_CREDIT)
    }

    /// Show the map in ui, which must be a child of the map's own rectangle.
    pub fn show(&mut self, ui: &mut egui::Ui, render_state: &egui_wgpu::RenderState) -> egui::Response {
        let response = self.map.show(ui, render_state);
        if let Some(record) = self.map.record() {
            self.record = Some(record.clone());
        }
        if let Some(error) = self.map.error() {
            self.error = Some(error.to_owned());
        }
        response
    }

    /// Move the camera without an animation, for a control or a double-click.
    pub fn jump_to(&mut self, centre: LatLon, zoom: f64) {
        self.map.command(Command::JumpTo(CameraChange {
            center: Some([centre.lon, centre.lat]),
            zoom: Some(zoom),
            ..CameraChange::default()
        }));
    }

    /// Drop the failed map and build it again at the last camera, keeping the style.
    pub fn retry(&mut self) {
        let mut inputs = self.map.inputs().clone();
        if let Some(record) = &self.record {
            inputs.camera = record.camera;
        }
        self.map = EguiMap::new(inputs);
        self.record = None;
        self.error = None;
    }
}

/// The colours the shipped land, sea and map chrome are painted with.
#[derive(Debug, Clone, Copy)]
pub struct MapPalette {
    /// The sea, and the background behind missing tiles.
    pub water: Color32,
    /// The filled land.
    pub land: Color32,
    /// The shoreline stroke.
    pub coast: Color32,
    /// Country boundaries.
    pub border: Color32,
    /// Rivers and lake edges.
    pub river: Color32,
    /// The latitude/longitude grid.
    pub graticule: Color32,
    /// The map's hairline border.
    pub frame: Color32,
    /// Small map labels.
    pub label: Color32,
    /// The credit line.
    pub credit: Color32,
}

/// The radius of a dot standing for count photographs.
pub fn dot_radius(count: usize) -> f32 {
    let count = count.max(1) as f32;
    MIN_DOT + DOT_GROWTH * count.sqrt()
}

/// The radius the map actually draws and hit-tests a pin at.
///
/// A cluster keeps the area-true radius it has always had, but never shrinks below a tappable disc; a single
/// photograph is a small thumbnail, so its square needs the reach of half its side. Keeping this beside
/// [\`dot_radius\`] means the painted shape and the clickable shape cannot disagree.
pub fn pin_radius(count: usize) -> f32 {
    if count <= 1 { 17.0 } else { dot_radius(count).clamp(15.0, MAX_DOT) }
}

/// The photographs a click opened, held so the map can paint their strip under the pins.
#[derive(Debug, Clone, PartialEq)]
pub struct Tray {
    /// The selected photographs, in the order the cluster held them.
    pub members: Vec<PhotoId>,
    /// The place the cluster stood for, so the strip can say where these were taken.
    pub centre: LatLon,
    /// A gazetteer name near that place, when the library's own file supplies one.
    pub name: Option<String>,
}

/// One dot on screen, and the cluster it stands for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot {
    /// The index of the cluster in the list the dots were built from.
    ///
    /// Carried explicitly rather than implied by position: a cluster that is off screen produces no dot, so a dot's
    /// index in the drawn list is **not** its cluster's index. The first version of the view assumed it was, which
    /// meant that with a photo in Sydney and the map over Brighton, clicking the one dot shown selected the wrong
    /// photographs.
    pub cluster: usize,
    /// Screen position, relative to the map's top-left.
    pub at: (f32, f32),
    /// Radius in pixels.
    pub radius: f32,
}

/// The dots to draw for a set of clusters, given a projection into map-relative pixels.
///
/// Clusters Mercator cannot place, and clusters outside the viewport, are left out — and each dot remembers which
/// cluster it came from, so the caller never has to assume the indices line up.
pub fn dots(clusters: &[sortlens_geo::Cluster], project: impl Fn(LatLon) -> Option<(f32, f32)>, width: f32, height: f32) -> Vec<Dot> {
    let mut dots = Vec::new();
    for (index, cluster) in clusters.iter().enumerate() {
        let Some((x, y)) = project(cluster.centre) else {
            continue;
        };
        if !(0.0..=width).contains(&x) || !(0.0..=height).contains(&y) {
            continue;
        }
        dots.push(Dot {
            cluster: index,
            at: (x, y),
            radius: pin_radius(cluster.len()),
        });
    }
    dots
}

/// Which dot a point landed on, if any.
///
/// Every dot within reach is a candidate and the **largest** wins, not the nearest: where a big dot overhangs a small
/// one, the person is aiming at the big one, and resolving by distance would hand the click to the sliver underneath.
/// Ties go to the earlier dot, so the answer is stable frame to frame.
pub fn dot_at(dots: &[Dot], at: (f32, f32)) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, dot) in dots.iter().enumerate() {
        let distance = ((at.0 - dot.at.0).powi(2) + (at.1 - dot.at.1).powi(2)).sqrt();
        // A couple of pixels of grace: a dot is small and a pointer is not a single pixel.
        if distance > dot.radius + 2.0 {
            continue;
        }
        match best {
            Some((_, best_radius)) if dot.radius <= best_radius => {}
            _ => best = Some((index, dot.radius)),
        }
    }
    best.map(|(index, _)| index)
}

/// The name to print beside a dot, if the gazetteer has one near enough.
///
/// None means "no place recorded near here", which is *not* the middle of the ocean and not an unknown name: the
/// dot then says its coordinates, because a coordinate is a true answer and an invented place name is not.
pub fn place_label(gazetteer: &sortlens_geo::Gazetteer, at: LatLon) -> Option<&sortlens_geo::Place> {
    gazetteer.nearest(at, LABEL_KM)
}

/// What the notice says when a dot is clicked.
///
/// Written as one function so the two forms cannot drift: a named dot says where it is by name and a coordinate, an
/// unnamed one says the coordinate alone rather than pretending to know a place.
#[must_use]
pub fn selection_notice(count: usize, at: LatLon, name: Option<&str>) -> String {
    let coordinates = format!(
        "{:.2}{} {:.2}{}",
        at.lat.abs(),
        if at.lat >= 0.0 { 'N' } else { 'S' },
        at.lon.abs(),
        if at.lon >= 0.0 { 'E' } else { 'W' }
    );
    match name {
        Some(name) => format!("Selected {count} photo(s) near {name} ({coordinates})"),
        None => format!("Selected {count} photo(s) at {coordinates}"),
    }
}

/// A zoom to actually use, given the set's range.
///
/// Zooming past the top of a tile set is clamped rather than refused: the tiles stop, the dots keep going, and the
/// view says which is which. A map that stops responding to the + button has not told anybody why.
pub fn clamp_zoom(wanted: u8, range: (u8, u8)) -> u8 {
    let (low, high) = range;
    // The bounds are put in order first: a set whose range arrives backwards is still a set, and clamp panics when
    // its minimum is above its maximum — an assertion failure in a paint path is not a way to learn that.
    let high = high.max(low).max(1);
    let low = low.max(1).min(high);
    wanted.clamp(low, high)
}

/// A gazetteer held between frames, with the file's stamp so a changed file is re-read.
///
/// The stamp is size and modification time: a file edited with the same length in the same nanosecond is not a case
/// worth a checksum on every frame, and reading tens of megabytes per frame to notice is not a case worth anything.
#[derive(Default)]
pub struct Names {
    loaded: Option<sortlens_geo::Gazetteer>,
    stamp: Option<(u64, Option<SystemTime>)>,
    error: Option<String>,
}

impl std::fmt::Debug for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Names")
            .field("places", &self.loaded.as_ref().map(sortlens_geo::Gazetteer::len))
            .field("error", &self.error)
            .finish()
    }
}

impl Clone for Names {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Names {
    /// An empty naming table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The gazetteer for a file, reloading it when the file has changed.
    ///
    /// None when there is no file at all. When the file exists but cannot be read, the reason is kept — the view
    /// shows it, because a places file that silently does nothing is worse than no places file.
    pub fn for_file(&mut self, path: Option<&Path>) -> Option<&sortlens_geo::Gazetteer> {
        let Some(path) = path else {
            self.loaded = None;
            self.stamp = None;
            self.error = None;
            return None;
        };
        let stamp = std::fs::metadata(path)
            .ok()
            .map(|metadata| (metadata.len(), metadata.modified().ok()));
        if stamp != self.stamp {
            self.stamp = stamp;
            self.error = None;
            match sortlens_geo::Gazetteer::load(path) {
                Ok(gazetteer) => self.loaded = Some(gazetteer),
                // A file that is not there is not a fault: most libraries have no places file, and a red line saying
                // so would teach the person to ignore the red line. The states worth reporting are a file that is
                // present and cannot be used.
                Err(sortlens_geo::GazetteerError::Missing(_)) => self.loaded = None,
                Err(error) => {
                    self.loaded = None;
                    self.error = Some(error.message());
                }
            }
        }
        self.loaded.as_ref()
    }

    /// Why the file could not be read, if it could not.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// How many place names are held.
    pub fn len(&self) -> usize {
        self.loaded.as_ref().map_or(0, sortlens_geo::Gazetteer::len)
    }

    /// Whether no name is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Decoded and styled tiles, kept between frames.
///
/// Both a tile that failed to decode and one the set simply does not carry are remembered, so neither is re-read on
/// every repaint — how a single bad tile, or a hole in an archive, becomes a stutter. They are not the same kind of
/// absence, though: a hole is painted as background and counted by neither [\`TileCache::decodes\`] nor
/// [\`TileCache::failures\`], because a partial map is normal and a corrupted tile is a fault.
#[derive(Default)]
pub struct TileCache {
    /// The basemap the entries came from. A different folder or archive clears the cache rather than painting one
    /// library's tiles over another's, which is a wrong map with no error message.
    source: Option<PathBuf>,
    entries: HashMap<(TileId, MapStyle), Option<TextureHandle>>,
    /// Counted so the cache limit has something to measure rather than a guess.
    decodes: usize,
    failures: usize,
}

/// Cloning would duplicate every texture handle without duplicating the GPU memory behind it, which is a cache that
/// looks free and is not. The cache is therefore moved with the state that owns it, never copied.
impl Clone for TileCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for TileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileCache")
            .field("entries", &self.entries.len())
            .field("decodes", &self.decodes)
            .field("failures", &self.failures)
            .finish()
    }
}

impl TileCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget every texture, for a style or source change.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.source = None;
        self.decodes = 0;
        self.failures = 0;
    }

    /// The texture for one tile in one style, decoding it on first sight.
    ///
    /// The bytes come from [\`TileSet::tile\`], so a folder and an archive are the same path here. None covers both
    /// "not there" and "not an image"; the view paints background either way, because a hole in a tile set is not an
    /// error to report, it is a hole.
    pub fn texture(&mut self, ctx: &egui::Context, set: &TileSet, id: TileId, style: MapStyle) -> Option<TextureHandle> {
        if self.source.as_deref() != Some(set.path()) {
            self.entries.clear();
            self.source = Some(set.path().to_path_buf());
        }
        let key = (id, style);
        if !self.entries.contains_key(&key) {
            if self.entries.len() >= CACHE_LIMIT {
                self.evict();
            }
            let loaded = match set.tile(id.zoom, id.x, id.y) {
                Some(bytes) => {
                    let texture = self.load(ctx, id, style, &bytes);
                    if texture.is_none() {
                        self.failures += 1;
                    }
                    texture
                }
                None => None,
            };
            self.entries.insert(key, loaded);
        }
        self.entries.get(&key).and_then(Clone::clone)
    }

    /// Whether a tile is already decoded, without loading it. Used by tests and by the view's own bookkeeping.
    pub fn contains(&self, id: TileId, style: MapStyle) -> bool {
        self.entries.contains_key(&(id, style))
    }

    /// How many tiles are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is held.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many tiles carrying bytes have been decoded, successes and failures together.
    ///
    /// A cell the set does not carry is not a decode: there were no bytes to read.
    pub fn decodes(&self) -> usize {
        self.decodes
    }

    /// How many tiles that carried bytes would not decode — a corrupted file, not a hole.
    pub fn failures(&self) -> usize {
        self.failures
    }

    fn load(&mut self, ctx: &egui::Context, id: TileId, style: MapStyle, bytes: &[u8]) -> Option<TextureHandle> {
        self.decodes += 1;
        if bytes.is_empty() || bytes.len() > MAX_TILE_BYTES as usize {
            return None;
        }
        let tile = sortlens_thumbs::decode_image(bytes, TILE_PX).ok()?;
        let mut image = ColorImage::from_rgba_unmultiplied([tile.width as usize, tile.height as usize], &tile.rgba);
        style.shade(&mut image);
        Some(ctx.load_texture(
            format!("tile:{}:{}:{}:{}", style.id(), id.zoom, id.x, id.y),
            image,
            egui::TextureOptions::LINEAR,
        ))
    }

    /// Drop half the cache. Textures are cheap to decode again and expensive to keep forever; halving avoids the
    /// thrash of evicting one entry per insertion once the limit is reached.
    fn evict(&mut self) {
        let doomed: Vec<(TileId, MapStyle)> = self.entries.keys().take(CACHE_LIMIT / 2).copied().collect();
        for cell in doomed {
            self.entries.remove(&cell);
        }
    }
}

/// The walkers tile source over a [\`TileSet\`].
///
/// Walkers owns panning, zooming and tile placement; this adapter owns only "give me the texture for this tile".
pub struct PlacesTiles<'a> {
    /// The folder or archive the tiles come from.
    pub set: &'a TileSet,
    /// The decoded textures, shared with the view so a style change does not decode everything twice.
    pub cache: &'a mut TileCache,
    /// The egui context the textures belong to.
    pub ctx: egui::Context,
    /// The concrete style to draw; Auto has already been resolved.
    pub style: MapStyle,
}

impl Tiles for PlacesTiles<'_> {
    fn at(&mut self, tile_id: TileId) -> Option<TilePiece> {
        let texture = self.cache.texture(&self.ctx, self.set, tile_id, self.style)?;
        Some(TilePiece::new(
            Tile::Raster(texture),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        ))
    }

    fn attribution(&self) -> Attribution {
        // The credit is painted by the view from the set's runtime attribution, which can be read from a file or an
        // archive. Walkers wants a 'static string, and borrowing the user's credit into the binary is not an option;
        // an empty value here keeps walkers from inventing one while the view still prints the real line.
        Attribution {
            text: "",
            url: "",
            logo_light: None,
            logo_dark: None,
        }
    }

    fn tile_size(&self) -> u32 {
        TILE_PX
    }
}

/// The zoom the map starts at before the first frame fits it to the library.
pub const DEFAULT_ZOOM: u8 = 5;

/// The spacing of the graticule at a zoom, in degrees.
///
/// A fixed grid cannot serve a world view and a city view at once: thirty degrees is a useful reference over a
/// continent and a wall of lines over a street. The step follows the scale, so the grid stays a reading aid as the
/// person zooms in instead of disappearing above zoom five.
fn grid_step(zoom: f64) -> f64 {
    if zoom <= 4.5 {
        30.0
    } else if zoom <= 6.5 {
        10.0
    } else if zoom <= 8.5 {
        5.0
    } else if zoom <= 10.5 {
        1.0
    } else if zoom <= 12.5 {
        0.5
    } else {
        0.1
    }
}

/// Project one polyline into the zoom-zero world square, unwrapping longitude as it goes.
///
/// Drawing the line directly would put a border that crosses the antimeridian on a path around the whole world. The
/// unwrap keeps each line local; the caller then paints the wrapped copies the viewport needs.
fn project_path(line: &[LatLon]) -> Vec<sortlens_geo::Point> {
    let mut points = Vec::with_capacity(line.len());
    let mut previous_x: Option<f64> = None;
    for at in line {
        let clamped = LatLon::new(at.lat.clamp(-sortlens_geo::MERCATOR_LIMIT, sortlens_geo::MERCATOR_LIMIT), at.lon);
        let Some(clamped) = clamped else { continue };
        let Some(mut point) = sortlens_geo::project(clamped, 0) else {
            continue;
        };
        if let Some(previous) = previous_x {
            while point.x - previous > sortlens_geo::TILE / 2.0 {
                point.x -= sortlens_geo::TILE;
            }
            while point.x - previous < -sortlens_geo::TILE / 2.0 {
                point.x += sortlens_geo::TILE;
            }
        }
        previous_x = Some(point.x);
        points.push(point);
    }
    if points.is_empty() {
        return points;
    }
    let middle = points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64;
    let shift = -(middle / sortlens_geo::TILE).floor() * sortlens_geo::TILE;
    for point in &mut points {
        point.x += shift;
    }
    points
}

/// Paint one already-projected path at every world wrap the viewport can see.
///
/// Each run of visible segments becomes one shape: a long river is one paint call, not one per segment. The cull is
/// generous because a segment can cross the viewport while both of its endpoints sit outside it.
fn paint_world_path(
    painter: &egui::Painter,
    map: egui::Rect,
    world: &[sortlens_geo::Point],
    to_screen: &dyn Fn(f64, f64) -> egui::Pos2,
    wrap_low: i32,
    wrap_high: i32,
    stroke: egui::Stroke,
) {
    if world.len() < 2 {
        return;
    }
    let visible = map.expand(32.0);
    for wrap in wrap_low..=wrap_high {
        let shift = f64::from(wrap) * sortlens_geo::TILE;
        let mut run: Vec<egui::Pos2> = Vec::new();
        for pair in world.windows(2) {
            let a = to_screen(pair[0].x + shift, pair[0].y);
            let b = to_screen(pair[1].x + shift, pair[1].y);
            if egui::Rect::from_points(&[a, b]).intersects(visible) {
                if run.is_empty() {
                    run.push(a);
                }
                run.push(b);
            } else {
                if run.len() > 1 {
                    painter.add(egui::Shape::line(std::mem::take(&mut run), stroke));
                }
                run.clear();
            }
        }
        if run.len() > 1 {
            painter.add(egui::Shape::line(run, stroke));
        }
    }
}

/// Fill one triangulated mesh in the zoom-zero world square at every visible wrap.
fn fill_world_mesh(
    painter: &egui::Painter,
    map: egui::Rect,
    triangles: &[[sortlens_geo::Point; 3]],
    colour: egui::Color32,
    centre_world: sortlens_geo::Point,
    scale: f64,
    wrap_low: i32,
    wrap_high: i32,
) {
    let mut mesh = egui::Mesh::default();
    let cull = map.expand(2.0);
    for triangle in triangles {
        for wrap in wrap_low..=wrap_high {
            let shift = f64::from(wrap) * sortlens_geo::TILE;
            let a = egui::pos2(
                map.center().x + ((triangle[0].x + shift - centre_world.x) * scale) as f32,
                map.center().y + ((triangle[0].y - centre_world.y) * scale) as f32,
            );
            let b = egui::pos2(
                map.center().x + ((triangle[1].x + shift - centre_world.x) * scale) as f32,
                map.center().y + ((triangle[1].y - centre_world.y) * scale) as f32,
            );
            let c = egui::pos2(
                map.center().x + ((triangle[2].x + shift - centre_world.x) * scale) as f32,
                map.center().y + ((triangle[2].y - centre_world.y) * scale) as f32,
            );
            if !egui::Rect::from_points(&[a, b, c]).intersects(cull) {
                continue;
            }
            let base = mesh.vertices.len() as u32;
            for point in [a, b, c] {
                mesh.colored_vertex(point, colour);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Paint the embedded land, lakes, borders, rivers and coastline for a map rectangle.
///
/// This is the offline basemap: the Natural Earth land polygons are filled, the 1:50m country boundaries, rivers and
/// lakes give it interior detail, the coastline is stroked over them, and a scale-aware graticule shows the
/// projection. Tiles, when supplied, replace all of it; the caller decides which one to draw. World coordinates are
/// stored at zoom zero, so one scale factor and one wrap offset put them anywhere the camera looks, including across
/// the antimeridian.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn paint_geography(
    painter: &egui::Painter,
    map: egui::Rect,
    style: MapStyle,
    tokens: &crate::theme::Tokens,
    centre: LatLon,
    zoom: f64,
) {
    let palette = style.palette(tokens);
    let Some(centre_world) = sortlens_geo::project(centre, 0) else {
        return;
    };
    let scale = zoom.exp2();
    let half_width = f64::from(map.width()) / (2.0 * scale);
    let min_x = centre_world.x - half_width;
    let max_x = centre_world.x + half_width;
    let wrap_low = (min_x / 256.0).floor() as i32;
    let wrap_high = (max_x / 256.0).floor() as i32;
    let to_screen = |x: f64, y: f64| {
        egui::pos2(
            map.center().x + ((x - centre_world.x) * scale) as f32,
            map.center().y + ((y - centre_world.y) * scale) as f32,
        )
    };

    painter.rect_filled(map, 0.0, palette.water);

    // The graticule is behind the land: a grid is a reading aid, not a thing on the ground. Its step follows the
    // scale, so it remains useful when the person zooms in rather than vanishing at the world view.
    let step = grid_step(zoom);
    let divisions = (180.0 / step).round() as i32;
    let grid = egui::Stroke::new(if step <= 1.0 { 0.4 } else { 0.5 }, palette.graticule);
    for index in -divisions..=divisions {
        let longitude = f64::from(index) * step;
        let x = (longitude + 180.0) / 360.0 * sortlens_geo::TILE;
        for wrap in wrap_low..=wrap_high {
            let sx = to_screen(x + f64::from(wrap) * sortlens_geo::TILE, 0.0).x;
            if sx >= map.left() - 1.0 && sx <= map.right() + 1.0 {
                painter.line_segment([egui::pos2(sx, map.top()), egui::pos2(sx, map.bottom())], grid);
            }
        }
    }
    for index in (-divisions + 1)..divisions {
        let latitude = f64::from(index) * step;
        let Some(world) = LatLon::new(latitude, 0.0).and_then(|at| sortlens_geo::project(at, 0)) else {
            continue;
        };
        let sy = to_screen(0.0, world.y).y;
        if sy >= map.top() - 1.0 && sy <= map.bottom() + 1.0 {
            painter.line_segment([egui::pos2(map.left(), sy), egui::pos2(map.right(), sy)], grid);
        }
    }

    // One mesh for all the land, so the sea shows between islands without a stroke around every triangle.
    fill_world_mesh(
        painter,
        map,
        sortlens_geo::land::Land::shipped().triangles(),
        palette.land,
        centre_world,
        scale,
        wrap_low,
        wrap_high,
    );

    // Lakes are water, not land: filled over the land mesh and outlined so a small lake still reads at world zoom.
    fill_world_mesh(
        painter,
        map,
        sortlens_geo::land::Land::lakes().triangles(),
        palette.water,
        centre_world,
        scale,
        wrap_low,
        wrap_high,
    );
    let lake_stroke = egui::Stroke::new(if zoom < 4.0 { 0.5 } else { 0.7 }, palette.coast);
    for polygon in sortlens_geo::land::Land::lakes().polygons() {
        paint_world_path(painter, map, &polygon.world, &to_screen, wrap_low, wrap_high, lake_stroke);
        for hole in &polygon.world_holes {
            paint_world_path(painter, map, hole, &to_screen, wrap_low, wrap_high, lake_stroke);
        }
    }

    // Country boundaries and rivers are the interior line work that turns a filled silhouette into a map. Both are
    // Natural Earth 1:50m and both are culled and wrapped like the coast.
    let border_stroke = egui::Stroke::new(if zoom < 4.0 { 0.55 } else { 0.75 }, palette.border);
    for line in sortlens_geo::lines::Lines::borders().lines() {
        let world = project_path(line);
        paint_world_path(painter, map, &world, &to_screen, wrap_low, wrap_high, border_stroke);
    }
    let river_stroke = egui::Stroke::new(if zoom < 4.0 { 0.45 } else { 0.65 }, palette.river);
    for line in sortlens_geo::lines::Lines::rivers().lines() {
        let world = project_path(line);
        paint_world_path(painter, map, &world, &to_screen, wrap_low, wrap_high, river_stroke);
    }

    // The coast is a stroke, not a filled edge: it is the line the land stops at, and it is what makes a schematic
    // map read as a map. Each run of visible segments is one shape, so a long coast is one paint call, not thousands.
    let stroke = egui::Stroke::new(if zoom < 3.0 { 0.7 } else { 1.0 }, palette.coast);
    for outline in sortlens_geo::coastline::Coastline::shipped().outlines() {
        let world = project_path(outline);
        paint_world_path(painter, map, &world, &to_screen, wrap_low, wrap_high, stroke);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn at(lat: f64, lon: f64) -> LatLon {
        LatLon::new(lat, lon).expect("a place")
    }

    #[test]
    fn the_graticule_tightens_as_the_map_zooms_in() {
        assert_eq!(grid_step(2.0), 30.0);
        assert_eq!(grid_step(6.0), 10.0);
        assert_eq!(grid_step(8.0), 5.0);
        assert_eq!(grid_step(10.0), 1.0);
        assert_eq!(grid_step(12.0), 0.5);
        assert_eq!(grid_step(14.0), 0.1);
        assert!(grid_step(4.6) < grid_step(4.5));
    }

    #[test]
    fn a_dot_grows_with_its_count_and_then_stops() {
        assert!(dot_radius(1) < dot_radius(4));
        assert!(dot_radius(4) < dot_radius(400));
        // Area, not width: four times the photographs is twice the radius, not four times.
        let single = dot_radius(1) - MIN_DOT;
        let quadruple = dot_radius(4) - MIN_DOT;
        assert!((quadruple / single - 2.0).abs() < 0.01, "got {}", quadruple / single);
        assert_eq!(dot_radius(1_000_000), MAX_DOT.max(dot_radius(1_000_000)));
        // A count of zero is one dot's worth, not a negative radius.
        assert_eq!(dot_radius(0), dot_radius(1));
    }

    #[test]
    fn a_single_photo_is_a_tappable_thumbnail_and_a_cluster_never_shrinks_away() {
        assert_eq!(pin_radius(0), pin_radius(1));
        assert!(pin_radius(1) > dot_radius(1), "an icon is wider than a hairline dot");
        assert!(pin_radius(2) >= 15.0, "two photographs are still a target");
        assert!(pin_radius(4) < pin_radius(400), "the bubble still grows with its count");
        assert_eq!(pin_radius(1_000_000), MAX_DOT, "and stops at a region");
    }

    #[test]
    fn a_click_finds_the_dot_under_it_and_prefers_the_big_one() {
        let dots = [
            Dot {
                cluster: 0,
                at: (10.0, 10.0),
                radius: 5.0,
            },
            Dot {
                cluster: 1,
                at: (100.0, 100.0),
                radius: 5.0,
            },
            Dot {
                cluster: 2,
                at: (12.0, 10.0),
                radius: 14.0,
            },
        ];
        assert_eq!(dot_at(&dots, (11.0, 10.0)), Some(2), "the big dot overhangs the small one");
        assert_eq!(dot_at(&dots, (100.0, 100.0)), Some(1));
        assert_eq!(dot_at(&dots, (60.0, 60.0)), None, "the sea is not a dot");
        // The grace of a couple of pixels: a click a pixel outside a radius-5 dot still lands on it.
        assert_eq!(dot_at(&dots, (100.0, 106.0)), Some(1));
        assert_eq!(dot_at(&dots, (100.0, 109.0)), None, "four pixels outside a radius-5 dot is the sea");
        assert_eq!(dot_at(&[], (0.0, 0.0)), None);
    }

    /// A cluster off screen produces no dot, and the dots that remain still point at the right clusters.
    #[test]
    fn a_dot_points_at_its_own_cluster_even_when_another_is_off_screen() {
        let brighton = at(50.8225, -0.1372);
        let sydney = at(-33.8688, 151.2093);
        let clusters = vec![
            sortlens_geo::Cluster {
                cell: (0, 0),
                members: vec![PhotoId(1)],
                centre: sydney,
                bounds: (-33.0, -34.0, 151.0, 152.0),
            },
            sortlens_geo::Cluster {
                cell: (1, 1),
                members: vec![PhotoId(2), PhotoId(3)],
                centre: brighton,
                bounds: (50.0, 51.0, -1.0, 0.0),
            },
        ];
        let project = |place: LatLon| {
            if place == sydney {
                Some((900.0, 100.0))
            } else {
                Some((400.0, 300.0))
            }
        };
        let dots = dots(&clusters, project, 800.0, 600.0);
        assert_eq!(dots.len(), 1, "Sydney is off screen and gets no dot");
        assert_eq!(dots[0].cluster, 1, "the dot remembers it is the Brighton cluster");
        assert_eq!(clusters[dots[0].cluster].members.len(), 2);
        let hit = dot_at(&dots, dots[0].at).expect("its own centre");
        assert_eq!(clusters[dots[hit].cluster].members, vec![PhotoId(2), PhotoId(3)]);
    }

    #[test]
    fn zooming_past_the_top_of_a_tile_set_is_clamped_not_refused() {
        assert_eq!(clamp_zoom(5, (2, 12)), 5);
        assert_eq!(clamp_zoom(30, (2, 12)), 12, "the tiles stop, the dots do not");
        assert_eq!(clamp_zoom(0, (2, 12)), 2);
        assert_eq!(clamp_zoom(9, (0, 0)), 1);
    }

    /// A dot borrows a name when the gazetteer has one near enough, and says its coordinate when it does not.
    #[test]
    fn a_name_is_borrowed_only_when_there_is_one_near_enough() {
        let gazetteer = sortlens_geo::Gazetteer::parse("Brighton\t51\t0\tGB\nLima\t-12\t-77\n");
        assert_eq!(gazetteer.len(), 2);
        let brighton = at(50.8225, -0.1372);
        let named = place_label(&gazetteer, brighton).expect("Brighton is 20 km away");
        assert_eq!(named.name, "Brighton");

        let ocean = at(25.0, -40.0);
        assert_eq!(place_label(&gazetteer, ocean), None);
        assert_eq!(
            selection_notice(42, brighton, Some(&named.name)),
            "Selected 42 photo(s) near Brighton (50.82N 0.14W)"
        );
        assert_eq!(selection_notice(1, ocean, None), "Selected 1 photo(s) at 25.00N 40.00W");
        let sydney = at(-33.8688, 151.2093);
        assert_eq!(selection_notice(3, sydney, None), "Selected 3 photo(s) at 33.87S 151.21E");
    }

    /// A gazetteer is parsed once and re-parsed only when the file changes.
    #[test]
    fn a_names_table_reloads_when_the_file_changes() {
        let dir = std::env::temp_dir().join(format!("sortlens-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("places.txt");

        let mut names = Names::new();
        assert!(names.for_file(None).is_none(), "no file, no names");
        assert!(names.is_empty());

        std::fs::write(&path, "Brighton\t51\t0\n").expect("write");
        assert_eq!(names.for_file(Some(&path)).map(sortlens_geo::Gazetteer::len), Some(1));
        assert_eq!(names.len(), 1);
        assert!(names.error().is_none());

        std::fs::write(&path, "Brighton\t51\t0\nLima\t-12\t-77\nHove\t51\t0\n").expect("write");
        assert_eq!(names.for_file(Some(&path)).map(sortlens_geo::Gazetteer::len), Some(3));

        std::fs::remove_file(&path).expect("remove");
        assert!(names.for_file(Some(&path)).is_none());
        assert!(names.is_empty());
        assert_eq!(names.error(), None, "no file is not a fault");

        let directory_instead = dir.join("places.txt");
        std::fs::create_dir_all(&directory_instead).expect("dir");
        names.for_file(Some(&directory_instead));
        assert!(
            names.error().is_some(),
            "a places path that is a directory is reported, not ignored"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A cache decodes a tile once, remembers a bad tile, and does not grow without end.
    #[test]
    fn a_tile_is_decoded_once_and_a_bad_one_is_remembered_as_bad() {
        let root = std::env::temp_dir().join(format!("sortlens-tilecache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("tiles");
        let good = folder.join("3").join("1");
        std::fs::create_dir_all(&good).expect("dir");
        std::fs::write(folder.join("attribution.txt"), "© Example Maps").expect("credit");
        std::fs::write(good.join("2.png"), sortlens_icon::png()).expect("write");
        let bad = folder.join("3").join("1").join("3.png");
        std::fs::write(&bad, b"not a png").expect("write");
        let set = TileSet::open(&root).expect("a folder with a credit");
        let png = TileId { zoom: 3, x: 1, y: 2 };
        let corrupt = TileId { zoom: 3, x: 1, y: 3 };
        let absent = TileId { zoom: 3, x: 1, y: 4 };

        let ctx = egui::Context::default();
        let mut cache = TileCache::new();
        // Uploading needs a frame in progress, so the loads happen inside one.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            assert!(cache.texture(ctx, &set, png, MapStyle::Original).is_some(), "a real png decodes");
            assert!(cache.texture(ctx, &set, png, MapStyle::Original).is_some());
            assert_eq!(cache.decodes(), 1, "the second look is a cache hit");
            assert!(cache.texture(ctx, &set, corrupt, MapStyle::Original).is_none());
            assert!(cache.texture(ctx, &set, corrupt, MapStyle::Original).is_none());
            assert_eq!(cache.decodes(), 2, "a failed tile is not re-decoded");
            assert_eq!(cache.failures(), 1);
            assert!(cache.texture(ctx, &set, absent, MapStyle::Original).is_none());
            assert_eq!(cache.len(), 3, "failures are remembered as failures");
            assert_eq!(cache.failures(), 1, "a hole is not a corrupted tile");
            assert_eq!(cache.decodes(), 2, "a hole is not a decode attempt");
            // The same bytes in a different style are a different texture: the style is part of the key.
            assert!(cache.texture(ctx, &set, png, MapStyle::Ink).is_some());
            assert_eq!(cache.decodes(), 3);
        });
        output.textures_delta.clear();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn online_styles_name_the_published_openfreemap_documents() {
        let urls: Vec<&str> = OpenFreeMapStyle::ALL.iter().map(|style| style.url()).collect();
        assert_eq!(
            urls,
            [
                "https://tiles.openfreemap.org/styles/liberty",
                "https://tiles.openfreemap.org/styles/bright",
                "https://tiles.openfreemap.org/styles/positron",
                "https://tiles.openfreemap.org/styles/dark",
                "https://tiles.openfreemap.org/styles/fiord",
            ]
        );
        for style in OpenFreeMapStyle::ALL {
            assert!(style.url().ends_with(style.id()), "{} names its own document", style.id());
            assert!(!style.label().is_empty());
        }
        assert!(OPENFREEMAP_CREDIT.contains("OpenStreetMap"));
    }

    #[test]
    fn the_default_online_style_follows_the_interface_luminance() {
        let charcoal = crate::theme::ThemeKind::Charcoal.tokens();
        let paper = crate::theme::ThemeKind::Paper.tokens();
        assert_eq!(OpenFreeMapStyle::auto(&charcoal), OpenFreeMapStyle::Dark);
        assert_eq!(OpenFreeMapStyle::auto(&paper), OpenFreeMapStyle::Liberty);
    }

    #[test]
    fn auto_style_follows_the_interface_and_a_tile_set_is_left_original() {
        let charcoal = crate::theme::ThemeKind::Charcoal.tokens();
        let paper = crate::theme::ThemeKind::Paper.tokens();
        assert_eq!(MapStyle::Auto.resolve(&charcoal, false), MapStyle::Ink);
        assert_eq!(MapStyle::Auto.resolve(&paper, false), MapStyle::Paper);
        assert_eq!(MapStyle::Auto.resolve(&charcoal, true), MapStyle::Original);
        assert_eq!(MapStyle::Ink.resolve(&paper, true), MapStyle::Ink, "an explicit style is kept");
    }

    #[test]
    fn every_style_changes_the_tile_and_the_four_inks_are_distinct() {
        let mut image = ColorImage::from_rgba_unmultiplied([2, 1], &[120, 140, 160, 255, 240, 240, 240, 255]);
        let original = image.clone();
        MapStyle::Ink.shade(&mut image);
        assert_ne!(image.pixels, original.pixels, "Ink restates the raster");
        let mut colours = Vec::new();
        for style in [MapStyle::Ink, MapStyle::Paper, MapStyle::Blueprint, MapStyle::Mono] {
            let mut sample = original.clone();
            style.shade(&mut sample);
            colours.push(sample.pixels[0]);
        }
        colours.dedup();
        assert_eq!(colours.len(), 4, "each style is its own ink");
        let mut untouched = original.clone();
        MapStyle::Original.shade(&mut untouched);
        assert_eq!(untouched.pixels, original.pixels);
    }

    /// The walkers adapter hands back the same texture the cache holds and never invents an attribution.
    #[test]
    fn the_walkers_adapter_uses_the_cache_and_leaves_the_credit_to_the_view() {
        let root = std::env::temp_dir().join(format!("sortlens-places-tiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("tiles");
        let good = folder.join("2").join("1");
        std::fs::create_dir_all(&good).expect("dir");
        std::fs::write(folder.join("attribution.txt"), "© Example Maps").expect("credit");
        std::fs::write(good.join("1.png"), sortlens_icon::png()).expect("write");
        let set = TileSet::open(&root).expect("a folder with a credit");
        let ctx = egui::Context::default();
        let mut cache = TileCache::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            let mut tiles = PlacesTiles {
                set: &set,
                cache: &mut cache,
                ctx: ctx.clone(),
                style: MapStyle::Original,
            };
            assert!(tiles.at(TileId { zoom: 2, x: 1, y: 1 }).is_some());
            assert_eq!(tiles.attribution().text, "");
            assert_eq!(tiles.tile_size(), TILE_PX);
        });
        output.textures_delta.clear();
        let _ = std::fs::remove_dir_all(&root);
    }
}
