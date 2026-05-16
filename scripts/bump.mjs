#!/usr/bin/env node
// Bump the project version using SemVer (MAJOR.MINOR.PATCH).
//   bump.mjs major  -> 4.0.5 -> 5.0.0
//   bump.mjs minor  -> 4.0.5 -> 4.1.0
//   bump.mjs patch  -> 4.0.5 -> 4.0.6
//
// Writes version.txt and the [package] version in Cargo.toml.
// The 3.x flow also updated tauri.conf.json, src-tauri/Cargo.toml,
// and root package.json; those files are gone in the 4.x rewrite
// (single Rust crate at the project root, no Tauri config, no
// frontend package.json).
//
// The release workflow rewrites the bundled Info.plist's
// CFBundleShortVersionString / CFBundleVersion from the tag at
// build time, so Info.plist isn't part of the bump set.

import { readFileSync, writeFileSync } from 'node:fs';

const mode = process.argv[2];
if (!['major', 'minor', 'patch'].includes(mode)) {
    console.error('usage: bump.mjs major|minor|patch');
    process.exit(1);
}

const VERSION_FILE = 'version.txt';
const CARGO_TOML = 'Cargo.toml';

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
// (e.g. 4.0.0-alpha.1 -> patch -> 4.0.0).
else if (isPrerelease) next = `${major}.${minor}.${patch}`;
else next = `${major}.${minor}.${patch + 1}`;

writeFileSync(VERSION_FILE, next + '\n');

// Cargo's [package] table allows only one `version = "..."` line;
// the section-scoped reducer below rewrites only that line so any
// `version = "..."` lines inside [dependencies] (there shouldn't be
// any, but defensively) aren't touched.
const cargo = readFileSync(CARGO_TOML, 'utf8');
const rewritten = cargo
    .split('\n')
    .reduce(
        (acc, line) => {
            let { lines, inPkg, done } = acc;
            if (line.startsWith('[')) {
                inPkg = line.trim() === '[package]';
            }
            if (inPkg && !done && /^version = ".*"$/.test(line)) {
                lines.push(`version = "${next}"`);
                done = true;
            } else {
                lines.push(line);
            }
            return { lines, inPkg, done };
        },
        { lines: [], inPkg: false, done: false },
    )
    .lines.join('\n');
writeFileSync(CARGO_TOML, rewritten);

console.log(`Version bumped (${mode}): ${current} -> ${next}`);
