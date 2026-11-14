let wasmPromise;

/**
 * Ares' `[Urban]` foot defaults and VERA's own urban-catalog filters, mirrored from
 * `src/map/rmg/tech_catalog.rs::resolve_vera_urban`. A host builds `urbanCatalog` by
 * resolving these names against its rules and art: a structure needs its footprint,
 * a foot is `known` when its rules section exists.
 */
export const VERA_URBAN_RULES = Object.freeze({
  /** Building-type prefixes the urban pass draws from. */
  structurePrefixes: Object.freeze(['CA', 'CT', 'CI']),
  /** Lower-cased `Name=` words that keep a building out of the civilian families. */
  setApartWords: Object.freeze([
    'wall',
    'fence',
    'gate',
    'corner',
    'middle',
    'gum',
    'barricade',
    'hedge',
    'light',
    'lamp',
    'pole',
    'sign',
    'barrel',
    'dumpster',
    'mail',
    'booth',
    'flag',
    'bench',
  ]),
  infantry: Object.freeze(['CIV1', 'CIV2', 'CIV3', 'CIVA', 'CIVB', 'CIVC']),
  vehicles: Object.freeze(['TRUCKA', 'TRUCKB', 'COP', 'EUROC', 'SUVW', 'SUVB', 'FTRK', 'AMBU']),
});

async function loadWasm() {
  if (wasmPromise === undefined) {
    wasmPromise = import('./wasm/vera20k_rmg_wasm.js')
      .then(async (module) => {
        await module.default();
        return module;
      })
      .catch((error) => {
        wasmPromise = undefined;
        throw error;
      });
  }
  return wasmPromise;
}

/** Initialize WASM before starting a generation job. Safe to call more than once. */
export async function initializeRandomMapWasm() {
  await loadWasm();
}

/** Run VERA's generator using the host's prepared rules and theater data. */
export async function generateRandomMap(input) {
  const { generate_random_map: generate } = await loadWasm();
  const output = generate(JSON.stringify(input));
  return JSON.parse(output);
}
