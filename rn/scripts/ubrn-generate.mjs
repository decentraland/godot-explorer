#!/usr/bin/env node
// Regenerates everything bound from the core's FFI (dcl_core and dcl_host), from a library that
// carries their uniffi metadata: libdclgodot of any target, or the small libdcl_bindings
// (`cargo build -p dcl_bindings`), which is the default:
//   modules/dcl-core/{src,cpp}/generated   TypeScript + C++ for the JSI TurboModule (ubrn)
//   modules/dcl-core/{android,ios}, src/{NativeDclCore,index}   the TurboModule glue (ubrn)
//   android/app/src/godot/java/.../host    Kotlin bindings of dcl_host, for the host (uniffi)
//   ios/DclRn/Generated                    Swift bindings of dcl_host, for the host (uniffi)
// A foreign trait has one vtable per process, so the JS shell implements dcl_core's traits and the
// host implements dcl_host's: each language gets the bindings of its own crate only.
// Usage: node scripts/ubrn-generate.mjs [path/to/library]
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, copyFileSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rn = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const lib = resolve(rn, '../lib');
const module = join(rn, 'modules/dcl-core');
const ubrn = join(rn, 'node_modules/uniffi-bindgen-react-native/bin/cli.cjs');

const hostLib =
  process.platform === 'darwin' ? 'libdcl_bindings.dylib' : 'libdcl_bindings.so';
const library = resolve(process.argv[2] ?? join(lib, 'target/debug', hostLib));
if (!existsSync(library)) {
  console.error(`missing ${library} (run: cd lib && cargo build -p dcl_bindings)`);
  process.exit(1);
}

const run = (command, args, cwd) => execFileSync(command, args, { cwd, stdio: 'inherit' });

// TypeScript + C++ bindings. Runs in the cargo workspace so uniffi finds the crate's uniffi.toml.
run(
  'node',
  [
    ubrn, 'generate', 'jsi', 'bindings', '--library', '--crate', 'dcl_core',
    '--ts-dir', join(module, 'src/generated'), '--cpp-dir', join(module, 'cpp/generated'), library,
  ],
  lib,
);
// ubrn emits every uniffi crate in the library; the shell binds only dcl_core, and registering
// dcl_host's traits from JS would take their vtable from the host.
for (const dir of ['src/generated', 'cpp/generated']) {
  for (const file of readdirSync(join(module, dir)).filter((f) => f.startsWith('dcl_host'))) {
    rmSync(join(module, dir, file));
  }
}
run('node', [ubrn, 'generate', 'jsi', 'turbo-module', '--config', 'ubrn.config.yaml', 'dcl_core'], module);

// Kotlin + Swift bindings for the host.
const tmp = join(rn, 'node_modules/.cache/dcl-core-native-bindings');
rmSync(tmp, { recursive: true, force: true });
mkdirSync(tmp, { recursive: true });
run('cargo', ['run', '-q', '-p', 'dcl_bindgen', '--', 'generate', '--library', library, '--crate', 'dcl_host', '--language', 'kotlin', '--language', 'swift', '--no-format', '--out-dir', tmp], lib);

const kotlinOut = join(rn, 'android/app/src/godot/java/org/decentraland/core/host');
mkdirSync(kotlinOut, { recursive: true });
copyFileSync(join(tmp, 'org/decentraland/core/host/dcl_host.kt'), join(kotlinOut, 'dcl_host.kt'));

const swiftOut = join(rn, 'ios/DclRn/Generated');
mkdirSync(swiftOut, { recursive: true });
for (const file of readdirSync(tmp).filter((f) => /^DclHostFfi/.test(f))) {
  copyFileSync(join(tmp, file), join(swiftOut, file));
}
