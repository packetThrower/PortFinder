#!/usr/bin/env node
// Read version.txt and create + push a v-prefixed git tag.
// The release workflow triggers on push of v* tags.

import { readFileSync } from 'node:fs';
import { execSync } from 'node:child_process';

const version = readFileSync('version.txt', 'utf8').trim();
const tag = `v${version}`;

console.log(`Tagging ${tag}…`);
execSync(`git tag ${tag}`, { stdio: 'inherit' });
execSync(`git push origin ${tag}`, { stdio: 'inherit' });
