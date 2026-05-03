// Copies the repo-root CHANGELOG.md into src/content/docs/changelog.md
// with Starlight frontmatter prepended. Single source of truth stays at
// repo root; the docs page is regenerated on every build.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, '../../CHANGELOG.md');
const dst = resolve(here, '../src/content/docs/changelog.md');

const body = readFileSync(src, 'utf8').replace(/^# Changelog\s*\n/, '');
const frontmatter = `---
title: Changelog
description: All notable changes per release.
editUrl: https://github.com/packetThrower/PortFinder/edit/main/CHANGELOG.md
---

`;

mkdirSync(dirname(dst), { recursive: true });
writeFileSync(dst, frontmatter + body);
console.log(`sync-changelog: wrote ${dst}`);
