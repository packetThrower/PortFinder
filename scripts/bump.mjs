#!/usr/bin/env node
// Bump the project version. Two modes:
//   bump.mjs day    -> set to today's date (YYYY.M.D)
//   bump.mjs patch  -> increment patch suffix (YYYY.M.D-N)
// Writes version.txt, src-tauri/Cargo.toml, and src-tauri/tauri.conf.json
// in lockstep.

import { readFileSync, writeFileSync } from 'node:fs';

const mode = process.argv[2];
if (mode !== 'day' && mode !== 'patch') {
    console.error('usage: bump.mjs day|patch');
    process.exit(1);
}

const VERSION_FILE = 'version.txt';
const CARGO_TOML = 'src-tauri/Cargo.toml';
const TAURI_CONF = 'src-tauri/tauri.conf.json';
const ROOT_PKG = 'package.json';

const current = readFileSync(VERSION_FILE, 'utf8').trim();

let next;
if (mode === 'day') {
    const now = new Date();
    next = `${now.getFullYear()}.${now.getMonth() + 1}.${now.getDate()}`;
} else {
    const m = current.match(/^(\d+\.\d+\.\d+)(?:-(\d+))?$/);
    if (!m) {
        console.error(`unrecognized version: ${current}`);
        process.exit(1);
    }
    const [, datePart, patchPart] = m;
    const nextPatch = patchPart ? Number(patchPart) + 1 : 1;
    next = `${datePart}-${nextPatch}`;
}

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

// Root package.json carries a version too — keep it in sync so
// `pnpm <script>` doesn't print a stale version banner.
const pkg = readFileSync(ROOT_PKG, 'utf8').replace(
    /"version": ".*"/,
    `"version": "${next}"`,
);
writeFileSync(ROOT_PKG, pkg);

console.log(`Version ${mode === 'day' ? 'bumped' : 'patched'} to ${next}`);
