use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vera20k_core::map::entities::EntityCategory;
use vera20k_core::map::map_file::{MapCell, MapHeader};
use vera20k_core::map::overlay::{OverlayEntry, TerrainObject};
use vera20k_core::map::rmg::RmgOptions;
use vera20k_core::map::rmg::build::{self, ResolvedTheaterInputs};
use vera20k_core::map::rmg::phases::shore::{SubTile, TileBlock, TileBlocks};
use vera20k_core::map::rmg::phases::tech_buildings::TechType;
use vera20k_core::map::rmg::phases::urban_areas::{
    UrbanCatalog, UrbanFoot, UrbanInputs, UrbanStructure,
};
use vera20k_core::map::rmg::phases::zones::OverlayZoneFacts;
use vera20k_core::map::rmg::rng::RmgRng;
use vera20k_core::map::rmg::settings::RmgSettings;
use vera20k_core::map::rmg::tiles::TileIds;
use vera20k_core::map::theater::TheaterCliffRanges;
use vera20k_core::util::retail_trig::TrigTable;
use wasm_bindgen::prelude::*;

/// Generate a map from already-resolved theater and rules data.
///
/// The caller owns asset and rules loading. `resolvedTheater`, `tileBlocks`,
/// and `techTypes` are the projection seam for host engines such as FA2Web.
#[wasm_bindgen]
pub fn generate_random_map(input_json: &str) -> Result<String, JsValue> {
    generate_random_map_json(input_json).map_err(|error| JsValue::from_str(&error))
}

fn generate_random_map_json(input_json: &str) -> Result<String, String> {
    let request: GenerateRequest = serde_json::from_str(input_json)
        .map_err(|error| format!("invalid random-map input: {error}"))?;
    let generated = generate(request)?;
    serde_json::to_string(&generated)
        .map_err(|error| format!("could not encode generated map: {error}"))
}

fn generate(request: GenerateRequest) -> Result<GeneratedMap, String> {
    let mut blocks = MemoryTileBlocks::default();
    for block in request.tile_blocks {
        if blocks.0.contains_key(&block.tile) {
            return Err(format!("duplicate tile block for tile {}", block.tile));
        }
        let width = usize::try_from(block.width)
            .ok()
            .filter(|width| *width > 0)
            .ok_or_else(|| format!("tile block {} has invalid width", block.tile))?;
        let height = usize::try_from(block.height)
            .ok()
            .filter(|height| *height > 0)
            .ok_or_else(|| format!("tile block {} has invalid height", block.tile))?;
        let expected_subtiles = width
            .checked_mul(height)
            .ok_or_else(|| format!("tile block {} dimensions overflow", block.tile))?;
        if block.subtiles.len() != expected_subtiles {
            return Err(format!(
                "tile block {} subtile count is {}, expected {expected_subtiles}",
                block.tile,
                block.subtiles.len()
            ));
        }
        let subtiles = block
            .subtiles
            .into_iter()
            .map(|subtile| {
                subtile.map(|subtile| SubTile {
                    height: subtile.height,
                    terrain: subtile.terrain,
                    slope: subtile.slope,
                })
            })
            .collect();
        blocks.0.insert(
            block.tile,
            TileBlock {
                width: block.width,
                height: block.height,
                subtiles,
            },
        );
    }

    let cliffs = request.resolved_theater.cliffs.into_native();
    let ids = request.resolved_theater.tile_ids.into_native(cliffs);
    let resolved = ResolvedTheaterInputs {
        ids,
        wheel_impassable: request.resolved_theater.wheel_impassable,
        cliff_back_impassability: request.resolved_theater.cliff_back_impassability,
        overlay_zones: request
            .resolved_theater
            .overlay_zones
            .into_iter()
            .map(Into::into)
            .collect(),
        morphable: request.resolved_theater.morphable,
        trig: Some(TrigTable::embedded()),
    };
    let settings = request.settings.into_native();
    let mut options = request.options.into_native(settings.generate_urban_areas);
    options.normalize();
    let urban_rng = RefCell::new(RmgRng::new(
        request.urban_main_seed.unwrap_or(options.seed_u16()),
    ));
    let urban_draw = || urban_rng.borrow_mut().next_u32();
    let urban_inputs = UrbanInputs {
        catalog: &settings.vera_urban,
        main_draw: &urban_draw,
    };
    let tech_types = request
        .tech_types
        .into_iter()
        .map(|tech| TechType {
            name: tech.name,
            footprint: tech
                .footprint
                .into_iter()
                .map(|cell| (cell[0], cell[1]))
                .collect(),
        })
        .collect::<Vec<_>>();

    let generated = build::generate_map(
        &options,
        &settings,
        &resolved,
        &blocks,
        &build::GenerationObjects {
            tech_types: &tech_types,
            urban_areas: Some(urban_inputs),
        },
    );
    Ok(GeneratedMap::from_native(
        generated.map_file,
        generated.start_waypoints,
        generated.unfilled_start_slots,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    options: OptionsInput,
    settings: SettingsInput,
    resolved_theater: ResolvedTheaterInput,
    tile_blocks: Vec<TileBlockInput>,
    tech_types: Vec<TechTypeInput>,
    #[serde(default)]
    urban_main_seed: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionsInput {
    theater: i32,
    map_type: i32,
    resources: i32,
    ruggedness: i32,
    time: i32,
    water_amount: i32,
    num_players: i32,
    tiberium: i32,
    tiberium_layout: i32,
    vegetation: i32,
    urban_presence: i32,
    urban_enabled: bool,
    width: i32,
    height: i32,
    accessibility: i32,
    region_size: i32,
    seed: i32,
    #[serde(default)]
    description: String,
    #[serde(default)]
    urban_areas: Option<bool>,
}

impl OptionsInput {
    fn into_native(self, default_urban_areas: bool) -> RmgOptions {
        RmgOptions {
            theater: self.theater,
            map_type: self.map_type,
            resources: self.resources,
            ruggedness: self.ruggedness,
            time: self.time,
            water_amount: self.water_amount,
            num_players: self.num_players,
            tiberium: self.tiberium,
            tiberium_layout: self.tiberium_layout,
            vegetation: self.vegetation,
            urban_presence: self.urban_presence,
            urban_enabled: self.urban_enabled,
            width: self.width,
            height: self.height,
            accessibility: self.accessibility,
            region_size: self.region_size,
            seed: self.seed,
            description: self.description.into(),
            urban_areas: self.urban_areas.unwrap_or(default_urban_areas),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SettingsInput {
    min_tiberium: i32,
    max_tiberium: i32,
    max_trees: i32,
    level_light: Option<Vec<i32>>,
    temperate_ambient: Option<Vec<i32>>,
    snow_ambient: Option<Vec<i32>>,
    temperate_rgb: Option<[Vec<i32>; 3]>,
    snow_rgb: Option<[Vec<i32>; 3]>,
    vegetation_min: Option<[i32; 5]>,
    vegetation_max: Option<[i32; 5]>,
    #[serde(default)]
    urban_catalog: UrbanCatalogInput,
    #[cfg(feature = "ares")]
    generate_urban_areas: Option<bool>,
    #[cfg(feature = "ares")]
    urban_structures: Option<Vec<String>>,
    #[cfg(feature = "ares")]
    urban_infantry: Option<Vec<String>>,
    #[cfg(feature = "ares")]
    urban_vehicles: Option<Vec<String>>,
}

impl SettingsInput {
    fn into_native(self) -> RmgSettings {
        let mut settings = RmgSettings::default();
        settings.min_tiberium = self.min_tiberium;
        settings.max_tiberium = self.max_tiberium;
        settings.max_trees = self.max_trees;
        if let Some(values) = self.level_light {
            settings.level_light = values;
        }
        if let Some(values) = self.temperate_ambient {
            settings.temperate_ambient = values;
        }
        if let Some(values) = self.snow_ambient {
            settings.snow_ambient = values;
        }
        if let Some(values) = self.temperate_rgb {
            settings.temperate_rgb = values;
        }
        if let Some(values) = self.snow_rgb {
            settings.snow_rgb = values;
        }
        if let Some(values) = self.vegetation_min {
            settings.vegetation_min = values;
        }
        if let Some(values) = self.vegetation_max {
            settings.vegetation_max = values;
        }
        settings.vera_urban = self.urban_catalog.into_native();
        #[cfg(feature = "ares")]
        {
            if let Some(value) = self.generate_urban_areas {
                settings.generate_urban_areas = value;
            }
            if let Some(values) = self.urban_structures {
                settings.urban_structures = values;
            }
            if let Some(values) = self.urban_infantry {
                settings.urban_infantry = values;
            }
            if let Some(values) = self.urban_vehicles {
                settings.urban_vehicles = values;
            }
        }
        settings
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UrbanCatalogInput {
    structures: Vec<UrbanStructureInput>,
    infantry: Vec<UrbanFootInput>,
    vehicles: Vec<UrbanFootInput>,
}

impl UrbanCatalogInput {
    fn into_native(self) -> UrbanCatalog {
        UrbanCatalog {
            structures: self
                .structures
                .into_iter()
                .map(|item| UrbanStructure {
                    name: item.name,
                    footprint: item
                        .footprint
                        .map(|cells| cells.into_iter().map(|[x, y]| (x, y)).collect()),
                })
                .collect(),
            infantry: self
                .infantry
                .into_iter()
                .map(|item| UrbanFoot {
                    name: item.name,
                    known: item.known,
                })
                .collect(),
            vehicles: self
                .vehicles
                .into_iter()
                .map(|item| UrbanFoot {
                    name: item.name,
                    known: item.known,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UrbanStructureInput {
    name: String,
    footprint: Option<Vec<[i16; 2]>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UrbanFootInput {
    name: String,
    known: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedTheaterInput {
    tile_ids: TileIdsInput,
    cliffs: CliffRangesInput,
    wheel_impassable: [bool; 16],
    cliff_back_impassability: u8,
    overlay_zones: Vec<OverlayZoneInput>,
    morphable: Vec<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayZoneInput {
    land: u8,
    early_land: bool,
    zone_type: Option<u8>,
}

impl From<OverlayZoneInput> for OverlayZoneFacts {
    fn from(input: OverlayZoneInput) -> Self {
        Self {
            land: input.land,
            early_land: input.early_land,
            zone_type: input.zone_type,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TileIdsInput {
    clear: i32,
    ramp_base: i32,
    ramp_smooth: i32,
    slope_set_pieces: i32,
    rough: i32,
    sand: i32,
    green: i32,
    rough_lat: i32,
    sand_lat: i32,
    green_lat: i32,
    pave_lat: i32,
    pave: i32,
    water_base: i32,
    shore: i32,
    water_bridge: i32,
    misc_pave: i32,
    paved_roads: i32,
    paved_road_ends: i32,
    medians: i32,
}

impl TileIdsInput {
    fn into_native(self, cliffs: TheaterCliffRanges) -> TileIds {
        TileIds {
            clear: self.clear,
            ramp_base: self.ramp_base,
            ramp_smooth: self.ramp_smooth,
            slope_set_pieces: self.slope_set_pieces,
            rough: self.rough,
            sand: self.sand,
            green: self.green,
            rough_lat: self.rough_lat,
            sand_lat: self.sand_lat,
            green_lat: self.green_lat,
            pave_lat: self.pave_lat,
            pave: self.pave,
            water_base: self.water_base,
            shore: self.shore,
            water_bridge: self.water_bridge,
            misc_pave: self.misc_pave,
            paved_roads: self.paved_roads,
            paved_road_ends: self.paved_road_ends,
            medians: self.medians,
            cliffs,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliffRangesInput {
    cliff_set: Option<u16>,
    cliff_ramps: Option<u16>,
    water_cliffs: Option<u16>,
    destroyable_cliffs: Option<u16>,
    bridge_set: Option<u16>,
    wood_bridge_set: Option<u16>,
    water_caves: Option<u16>,
    waterfall_east: Option<u16>,
    waterfall_west: Option<u16>,
    waterfall_north: Option<u16>,
    waterfall_south: Option<u16>,
}

impl CliffRangesInput {
    fn into_native(self) -> TheaterCliffRanges {
        TheaterCliffRanges {
            cliff_set: self.cliff_set,
            cliff_ramps: self.cliff_ramps,
            water_cliffs: self.water_cliffs,
            destroyable_cliffs: self.destroyable_cliffs,
            bridge_set: self.bridge_set,
            wood_bridge_set: self.wood_bridge_set,
            water_caves: self.water_caves,
            waterfall_east: self.waterfall_east,
            waterfall_west: self.waterfall_west,
            waterfall_north: self.waterfall_north,
            waterfall_south: self.waterfall_south,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TileBlockInput {
    tile: i32,
    width: i32,
    height: i32,
    subtiles: Vec<Option<SubTileInput>>,
}

#[derive(Debug, Deserialize)]
struct SubTileInput {
    height: u8,
    terrain: u8,
    slope: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TechTypeInput {
    name: String,
    footprint: Vec<[i16; 2]>,
}

#[derive(Debug, Default)]
struct MemoryTileBlocks(HashMap<i32, TileBlock>);

impl TileBlocks for MemoryTileBlocks {
    fn block(&self, tile: i32) -> Option<&TileBlock> {
        self.0.get(&tile)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedMap {
    map: GeneratedMapData,
    player_starts: Vec<PlayerStart>,
    unfilled_start_slots: usize,
}

impl GeneratedMap {
    fn from_native(
        map: vera20k_core::map::map_file::MapFile,
        starts: Vec<(u8, u16, u16)>,
        unfilled_start_slots: usize,
    ) -> Self {
        let structures = map
            .entities
            .into_iter()
            .map(|entity| StructureOutput {
                owner: entity.owner,
                type_id: entity.type_id,
                health: entity.health,
                x: entity.cell_x,
                y: entity.cell_y,
                facing: entity.facing,
                category: entity.category,
            })
            .collect();
        Self {
            map: GeneratedMapData {
                header: map.header.into(),
                cells: map.cells.into_iter().map(Into::into).collect(),
                overlays: map.overlays.into_iter().map(Into::into).collect(),
                terrain_objects: map.terrain_objects.into_iter().map(Into::into).collect(),
                structures,
                ini: serde_json::to_value(map.ini)
                    .expect("IniFile has a serializable stored-section representation"),
            },
            player_starts: starts
                .into_iter()
                .map(|(player, x, y)| PlayerStart { player, x, y })
                .collect(),
            unfilled_start_slots,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedMapData {
    header: MapHeaderOutput,
    cells: Vec<MapCellOutput>,
    overlays: Vec<OverlayOutput>,
    terrain_objects: Vec<TerrainObjectOutput>,
    structures: Vec<StructureOutput>,
    ini: serde_json::Value,
}

impl From<MapHeader> for MapHeaderOutput {
    fn from(header: MapHeader) -> Self {
        Self {
            theater: header.theater,
            fill: header.fill,
            level: header.level,
            width: header.width,
            height: header.height,
            local_left: header.local_left,
            local_top: header.local_top,
            local_width: header.local_width,
            local_height: header.local_height,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MapHeaderOutput {
    theater: String,
    fill: String,
    level: i32,
    width: u32,
    height: u32,
    local_left: u32,
    local_top: u32,
    local_width: u32,
    local_height: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MapCellOutput {
    rx: u16,
    ry: u16,
    tile_index: i32,
    sub_tile: u8,
    z: u8,
    legacy_ice_byte: u8,
}

impl From<MapCell> for MapCellOutput {
    fn from(cell: MapCell) -> Self {
        Self {
            rx: cell.rx,
            ry: cell.ry,
            tile_index: cell.tile_index,
            sub_tile: cell.sub_tile,
            z: cell.z,
            legacy_ice_byte: cell.legacy_ice_byte,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlayOutput {
    rx: u16,
    ry: u16,
    overlay_id: u8,
    frame: u8,
}

impl From<OverlayEntry> for OverlayOutput {
    fn from(entry: OverlayEntry) -> Self {
        Self {
            rx: entry.rx,
            ry: entry.ry,
            overlay_id: entry.overlay_id,
            frame: entry.frame,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerrainObjectOutput {
    rx: u16,
    ry: u16,
    name: String,
}

impl From<TerrainObject> for TerrainObjectOutput {
    fn from(object: TerrainObject) -> Self {
        Self {
            rx: object.rx,
            ry: object.ry,
            name: object.name,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StructureOutput {
    owner: String,
    type_id: String,
    health: i32,
    x: u16,
    y: u16,
    facing: u8,
    category: EntityCategory,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerStart {
    player: u8,
    x: u16,
    y: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_projects_a_map_and_keeps_the_player_starts() {
        let generated = generate(sample_request()).expect("valid prepared inputs");

        assert_eq!(
            (generated.map.header.width, generated.map.header.height),
            (74, 82)
        );
        assert!(!generated.map.cells.is_empty());
        assert_eq!(generated.player_starts.len(), 4);
        assert_eq!(generated.unfilled_start_slots, 0);
        assert_eq!(generated.map.header.theater, "TEMPERATE");
    }

    #[test]
    fn rejects_tile_blocks_with_incomplete_subtile_data() {
        let mut request = sample_request();
        request.tile_blocks[0].subtiles.clear();

        match generate(request) {
            Err(error) => assert!(error.contains("subtile count")),
            Ok(_) => panic!("incomplete TMP blocks must be rejected before generation"),
        }
    }

    #[test]
    fn public_entry_reports_malformed_json_as_a_recoverable_error() {
        let error = generate_random_map_json("{").expect_err("malformed input must be rejected");

        assert!(error.starts_with("invalid random-map input:"));
    }

    #[test]
    fn ares_urban_options_and_resolved_catalog_reach_the_core() {
        let mut request = sample_request();
        request.options.urban_areas = None;
        request.settings.generate_urban_areas = Some(true);
        request.settings.urban_structures = Some(vec!["CABUNK01".to_string()]);
        request.settings.level_light = Some(vec![0, 1, 2, 3]);
        request.settings.vegetation_min = Some([10, 20, 30, 40, 50]);
        request.settings.urban_catalog.structures = vec![UrbanStructureInput {
            name: "CABUNK01".to_string(),
            footprint: Some(vec![[0, 0], [1, 0], [0, 1], [1, 1]]),
        }];

        let settings = request.settings.into_native();
        let options = request.options.into_native(settings.generate_urban_areas);

        assert!(options.urban_areas);
        assert!(settings.generate_urban_areas);
        assert_eq!(settings.urban_structures, ["CABUNK01"]);
        assert_eq!(settings.level_light, [0, 1, 2, 3]);
        assert_eq!(settings.vegetation_min, [10, 20, 30, 40, 50]);
        assert_eq!(settings.vera_urban.structures[0].name, "CABUNK01");
        assert_eq!(
            settings.vera_urban.structures[0].footprint.as_deref(),
            Some(&[(0, 0), (1, 0), (0, 1), (1, 1)][..])
        );

        let mut explicit_off = sample_request().options;
        explicit_off.urban_areas = Some(false);
        assert!(!explicit_off.into_native(true).urban_areas);
    }

    #[test]
    fn extended_size_and_player_options_survive_the_public_input_projection() {
        let mut input = sample_request().options;
        input.map_type = 0;
        input.theater = 4;
        input.num_players = 30;
        input.width = 7;
        input.height = 7;
        input.urban_presence = 80;
        input.urban_enabled = true;

        let mut options = input.into_native(false);
        options.normalize();

        assert_eq!(options.num_players, 30);
        assert_eq!((options.width, options.height), (7, 7));
        assert!(options.urban_enabled);
        assert_eq!(
            vera20k_core::map::rmg::generated_dimensions(&options),
            (500, 500)
        );
    }

    fn sample_request() -> GenerateRequest {
        let make_block = |tile| TileBlockInput {
            tile,
            width: 1,
            height: 1,
            subtiles: vec![Some(SubTileInput {
                height: 0,
                terrain: 0,
                slope: 0,
            })],
        };
        GenerateRequest {
            options: OptionsInput {
                theater: 0,
                map_type: 1,
                resources: 1,
                ruggedness: 20,
                time: 1,
                water_amount: 30,
                num_players: 4,
                tiberium: 50,
                tiberium_layout: 0,
                vegetation: 50,
                urban_presence: 0,
                urban_enabled: false,
                width: 0,
                height: 0,
                accessibility: 0,
                region_size: 0,
                seed: 1234,
                description: String::new(),
                urban_areas: None,
            },
            settings: SettingsInput {
                min_tiberium: 2500,
                max_tiberium: 5500,
                max_trees: 500,
                ..SettingsInput::default()
            },
            resolved_theater: ResolvedTheaterInput {
                tile_ids: TileIdsInput {
                    clear: 0,
                    ramp_base: 600,
                    ramp_smooth: 620,
                    slope_set_pieces: -1,
                    rough: 700,
                    sand: 800,
                    green: 100,
                    rough_lat: 710,
                    sand_lat: 810,
                    green_lat: 110,
                    pave_lat: -1,
                    pave: -1,
                    water_base: 500,
                    shore: 400,
                    water_bridge: -1,
                    misc_pave: -1,
                    paved_roads: -1,
                    paved_road_ends: -1,
                    medians: -1,
                },
                cliffs: CliffRangesInput {
                    cliff_set: None,
                    cliff_ramps: None,
                    water_cliffs: None,
                    destroyable_cliffs: None,
                    bridge_set: None,
                    wood_bridge_set: None,
                    water_caves: None,
                    waterfall_east: None,
                    waterfall_west: None,
                    waterfall_north: None,
                    waterfall_south: None,
                },
                wheel_impassable: [false; 16],
                cliff_back_impassability: 2,
                overlay_zones: Vec::new(),
                morphable: vec![false; 900],
            },
            tile_blocks: [0, 100, 110, 400, 500, 600, 620, 700, 710, 800, 810]
                .into_iter()
                .map(make_block)
                .collect(),
            tech_types: Vec::new(),
            urban_main_seed: None,
        }
    }
}
