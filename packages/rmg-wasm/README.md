# `@vera20k/rmg-wasm`

TypeScript entry point for VERA20k's Random Map Generator. The package wraps the existing Rust
generator as browser WebAssembly; it does not include game assets or duplicate generation logic.

The host loads its own game installation and prepares the resolved theater/rules input. This keeps
FA2Web's existing VFS authoritative for loose-file and archive overrides. The package takes the
prepared data in [`index.d.ts`](index.d.ts) and returns typed map geometry, placements, and starts.
FA2Web can project that result into its `MapDocument` and use its existing `.map` serializer.

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
