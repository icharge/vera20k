let wasmPromise;

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
