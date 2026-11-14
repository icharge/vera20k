# `@vera20k/rmg-wasm`

TypeScript entry point for VERA20k's Random Map Generator. The package wraps the existing Rust
generator as browser WebAssembly; it does not include game assets or duplicate generation logic.

The host loads its own game installation and prepares the resolved theater/rules input. This keeps
FA2Web's existing VFS authoritative for loose-file and archive overrides. The package compiles the
current VERA20k generator, including its 30-player and 500×500 size support, Archipelago and the
Urban, Desert, NewUrban and Lunar theaters, advanced sliders, Urban Presence towns, and Ares Urban
Areas support. The host passes these settings and resolved theater data through the types in
[`index.d.ts`](index.d.ts); the result contains typed map geometry, placements, and starts. FA2Web
can project that result into its `MapDocument` and use its existing `.map` serializer.

`VERA_URBAN_RULES` carries the urban catalog's own selection rules — the Ares `[Urban]` foot
defaults and the building prefixes and `Name=` words the urban pass excludes — so a host can build
`settings.urbanCatalog` from its rules and art without duplicating them.

The caller supplies full `RMGMD.INI`-derived settings when available. Missing optional lighting,
vegetation, and urban fields use the core constructor defaults. For reproducible urban-foot choices,
set `urbanMainSeed`; it defaults to the map seed and drives a separate RNG stream.

## Build and use locally

From the VERA20K repository root, build the WASM package:

```sh
python packages/rmg-wasm/build.py
```

Then pack it for a local FA2Web trial:

```sh
cd packages/rmg-wasm
npm pack
```

Install the resulting tarball in FA2Web with `pnpm add <path-to-tarball>`. No registry publish is
needed for the trial. The WASM glue loads its `.wasm` next to the generated JS module; Vite handles
that package-relative asset URL.

```ts
import { generateRandomMap } from '@vera20k/rmg-wasm';

const generated = await generateRandomMap(preparedInputs);
```

The generation call is synchronous inside WASM. Invoke it in FA2Web's existing random-map worker.
The package reports malformed input and duplicate tile blocks as errors; it does not expose a
partially generated map.
