export interface RmgOptions {
  theater: number;
  mapType: number;
  resources: number;
  ruggedness: number;
  time: number;
  waterAmount: number;
  numPlayers: number;
  tiberium: number;
  tiberiumLayout: number;
  vegetation: number;
  urbanPresence: number;
  urbanEnabled: boolean;
  width: number;
  height: number;
  accessibility: number;
  regionSize: number;
  seed: number;
  description?: string;
}

export interface RmgSettings {
  minTiberium: number;
  maxTiberium: number;
  maxTrees: number;
}

export interface TheaterCliffRanges {
  cliffSet?: number | null;
  cliffRamps?: number | null;
  waterCliffs?: number | null;
  destroyableCliffs?: number | null;
  bridgeSet?: number | null;
  woodBridgeSet?: number | null;
  waterCaves?: number | null;
  waterfallEast?: number | null;
  waterfallWest?: number | null;
  waterfallNorth?: number | null;
  waterfallSouth?: number | null;
}

export interface RmgTileIds {
  clear: number;
  rampBase: number;
  rampSmooth: number;
  slopeSetPieces: number;
  rough: number;
  sand: number;
  green: number;
  roughLat: number;
  sandLat: number;
  greenLat: number;
  paveLat: number;
  pave: number;
  waterBase: number;
  shore: number;
  waterBridge: number;
  miscPave: number;
  pavedRoads: number;
  pavedRoadEnds: number;
  medians: number;
}

export interface ResolvedTheaterInputs {
  tileIds: RmgTileIds;
  cliffs: TheaterCliffRanges;
  /** Sixteen LandType flags in the VERA core's canonical table order. */
  wheelImpassable: readonly [
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
  ];
  cliffBackImpassability: number;
  /** Ordered overlay facts, indexed by overlay id. */
  overlayZones: readonly { land: number; earlyLand: boolean; zoneType: number | null }[];
  /** `Morphable=` per flat theater tile index. */
  morphable: readonly boolean[];
}

export interface RmgSubTile {
  height: number;
  terrain: number;
  slope: number;
}

export interface RmgTileBlock {
  tile: number;
  width: number;
  height: number;
  /** Row-major subtiles; null entries represent holes in the TMP block. */
  subtiles: readonly (RmgSubTile | null)[];
}

export interface RmgTechType {
  name: string;
  /** Foundation offsets relative to the building's north-west anchor. */
  footprint: readonly (readonly [number, number])[];
}

export interface GenerateRandomMapInput {
  options: RmgOptions;
  settings: RmgSettings;
  resolvedTheater: ResolvedTheaterInputs;
  tileBlocks: readonly RmgTileBlock[];
  techTypes: readonly RmgTechType[];
}

export interface GeneratedMapHeader {
  theater: string;
  fill: string;
  level: number;
  width: number;
  height: number;
  localLeft: number;
  localTop: number;
  localWidth: number;
  localHeight: number;
}

export interface GeneratedMapCell {
  rx: number;
  ry: number;
  tileIndex: number;
  subTile: number;
  z: number;
  legacyIceByte: number;
}

export interface GeneratedMapOverlay {
  rx: number;
  ry: number;
  overlayId: number;
  frame: number;
}

export interface GeneratedMapTerrainObject {
  rx: number;
  ry: number;
  name: string;
}

export interface GeneratedMapStructure {
  owner: string;
  typeId: string;
  health: number;
  x: number;
  y: number;
  facing: number;
  category: 'Unit' | 'Infantry' | 'Structure' | 'Aircraft';
}

export interface GeneratedMapIni {
  sections: readonly {
    name: string;
    entries: Readonly<Record<string, string>>;
    keyOrder: readonly string[];
    projectedValues: Readonly<Record<string, readonly string[]>>;
  }[];
}

export interface GeneratedMapData {
  header: GeneratedMapHeader;
  cells: readonly GeneratedMapCell[];
  overlays: readonly GeneratedMapOverlay[];
  terrainObjects: readonly GeneratedMapTerrainObject[];
  structures: readonly GeneratedMapStructure[];
  ini: GeneratedMapIni;
}

export interface GeneratedPlayerStart {
  player: number;
  x: number;
  y: number;
}

export interface GenerateRandomMapResult {
  map: GeneratedMapData;
  playerStarts: readonly GeneratedPlayerStart[];
  unfilledStartSlots: number;
}

/** Load the WASM module once. Calling this is optional; generation initializes it lazily. */
export function initializeRandomMapWasm(): Promise<void>;

/** Generate a map from host-resolved retail rules and theater data. */
export function generateRandomMap(
  input: GenerateRandomMapInput,
): Promise<GenerateRandomMapResult>;
