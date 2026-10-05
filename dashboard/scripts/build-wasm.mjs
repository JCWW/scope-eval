// Build the simulation engine (crates/scope-sim-wasm) to WebAssembly and
// generate its JavaScript bindings into src/wasm/.
//
// Needs: a Rust toolchain with the wasm32-unknown-unknown target, and
// wasm-bindgen-cli at the same version as the wasm-bindgen crate:
//   rustup target add wasm32-unknown-unknown
//   cargo install wasm-bindgen-cli --version 0.2.129 --locked
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const outDir = path.resolve(here, '..', 'src', 'wasm');
const wasm = path.join(repo, 'target', 'wasm32-unknown-unknown', 'release', 'scope_sim_wasm.wasm');

function run(cmd, args) {
  console.log(`> ${cmd} ${args.join(' ')}`);
  try {
    execFileSync(cmd, args, { cwd: repo, stdio: 'inherit' });
  } catch (e) {
    if (e.code === 'ENOENT') {
      console.error(`\n'${cmd}' was not found. See dashboard/README.md, "Prerequisites".`);
    }
    process.exit(1);
  }
}

run('cargo', ['build', '-p', 'scope-sim-wasm', '--target', 'wasm32-unknown-unknown', '--release']);
run('wasm-bindgen', ['--target', 'web', '--out-dir', outDir, wasm]);
