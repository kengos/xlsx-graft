// Run by `make sdk:verify:js` after it installs the packed tarball into this
// project, copied out of the checkout: loads the package from there with
// `import` and with `require()`, checks what the tarball holds, and that it
// reaches the engine on the gated line's Node.js.
//
//     node check.mjs <this project's dir> <engine/Cargo.toml> <Node.js release>

import { readdirSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as imported from 'xlsx-graft';

const [project, cargoToml, release] = process.argv.slice(2);
const installed = join(project, 'node_modules', 'xlsx-graft');

function check(ok, message) {
  if (!ok) {
    console.error(message);
    process.exit(1);
  }
}

function files(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => relative(installed, join(entry.parentPath, entry.name)))
    .sort();
}

const engine = /^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms.exec(
  readFileSync(cargoToml, 'utf8'),
)?.[1];
check(engine, `${cargoToml} has no [workspace.package] version`);

check(
  process.versions.node === release,
  `the package was loaded on Node.js ${process.versions.node}, not on the gated ${release}`,
);

const resolved = fileURLToPath(import.meta.resolve('xlsx-graft'));
check(
  resolved === join(installed, 'dist', 'index.js'),
  `imported xlsx-graft from ${resolved}, not from ${installed}`,
);

// The package ships ESM alone; require() loads it through Node.js's
// require(esm), and gets the same module.
const required = createRequire(join(project, 'package.json'))('xlsx-graft');
check(required.version === imported.version, 'require() did not load the module import did');

const manifest = JSON.parse(readFileSync(join(installed, 'package.json'), 'utf8'));
check(manifest.version === engine, `xlsx-graft is ${manifest.version}, the engine is ${engine}`);
for (const field of ['dependencies', 'optionalDependencies', 'peerDependencies']) {
  check(
    manifest[field] === undefined,
    `xlsx-graft declares ${field}: ${JSON.stringify(manifest[field])}`,
  );
}
// The tarball was installed with --ignore-scripts, so a hook that would run
// on a user's install (and could download) is looked for by name.
const hooks = ['preinstall', 'install', 'postinstall', 'prepare'].filter(
  (hook) => manifest.scripts?.[hook] !== undefined,
);
check(hooks.length === 0, `xlsx-graft runs ${hooks.join(', ')} when installed`);

// No addon until a release ships one; the compiled sources and their types,
// the README and the license files, and nothing else.
const shipped = files(installed);
const expected = ['LICENSE-APACHE', 'LICENSE-BSD', 'LICENSE-MIT', 'README.md', 'package.json'];
const unexpected = shipped.filter(
  (file) => !expected.includes(file) && !/^dist\/[^/]+\.(?:js|d\.ts)$/.test(file),
);
check(unexpected.length === 0, `the tarball ships ${unexpected.join(', ')}`);
for (const file of [...expected, 'dist/index.js', 'dist/index.d.ts']) {
  check(shipped.includes(file), `the tarball ships no ${file}`);
}

const result = imported.version();
check(result.success, `version() failed: ${JSON.stringify(result.diagnostics)}`);
check(result.value === engine, `version() is ${result.value}, the engine is ${engine}`);

console.log(
  `installed xlsx-graft ${manifest.version} from ${installed} loads engine ${result.value}` +
    ` on Node.js ${release}, by import and require()`,
);
