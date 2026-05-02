#!/usr/bin/env node
// Bump the project version using SemVer (MAJOR.MINOR.PATCH).
//   bump.mjs major  -> 3.4.5 -> 4.0.0
//   bump.mjs minor  -> 3.4.5 -> 3.5.0
//   bump.mjs patch  -> 3.4.5 -> 3.4.6
// Writes version.txt, src-tauri/Cargo.toml, src-tauri/tauri.conf.json,
// and root package.json in lockstep.

import { readFileSync, writeFileSync } from 'node:fs';

const mode = process.argv[2];
if (!['major', 'minor', 'patch'].includes(mode)) {
    console.error('usage: bump.mjs major|minor|patch');
    process.exit(1);
}

const VERSION_FILE = 'version.txt';
const CARGO_TOML = 'src-tauri/Cargo.toml';
const TAURI_CONF = 'src-tauri/tauri.conf.json';
const ROOT_PKG = 'package.json';

const current = readFileSync(VERSION_FILE, 'utf8').trim();
// Accept full SemVer 2 (MAJOR.MINOR.PATCH with optional -prerelease).
// The pre-release suffix is dropped on bump — bumping a pre-release
// always graduates to the next stable, not the next pre-release.
const m = current.match(/^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?$/);
if (!m) {
    console.error(`unrecognized version: ${current} (expected MAJOR.MINOR.PATCH[-prerelease])`);
    process.exit(1);
}

const [, majorStr, minorStr, patchStr] = m;
const major = Number(majorStr);
const minor = Number(minorStr);
const patch = Number(patchStr);
const isPrerelease = current.includes('-');

let next;
if (mode === 'major') next = `${major + 1}.0.0`;
else if (mode === 'minor') next = `${major}.${minor + 1}.0`;
// Patch bump from a pre-release graduates to the same X.Y.Z, not Z+1
// (e.g. 3.3.0-alpha.1 -> patch -> 3.3.0).
else if (isPrerelease) next = `${major}.${minor}.${patch}`;
else next = `${major}.${minor}.${patch + 1}`;

writeFileSync(VERSION_FILE, next + '\n');

const cargo = readFileSync(CARGO_TOML, 'utf8').replace(
    /^version = ".*"$/m,
    `version = "${next}"`,
);
writeFileSync(CARGO_TOML, cargo);

const conf = readFileSync(TAURI_CONF, 'utf8').replace(
    /"version": ".*"/,
    `"version": "${next}"`,
);
writeFileSync(TAURI_CONF, conf);

const pkg = readFileSync(ROOT_PKG, 'utf8').replace(
    /"version": ".*"/,
    `"version": "${next}"`,
);
writeFileSync(ROOT_PKG, pkg);

console.log(`Version bumped (${mode}): ${current} -> ${next}`);
