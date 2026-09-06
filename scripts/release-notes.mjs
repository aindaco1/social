#!/usr/bin/env node
// One changelog entry owns both GitHub release notes and in-app updater notes.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

export function extractReleaseNotes(changelog, version) {
    if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Release notes require a stable desktop version');
    const escaped = version.replaceAll('.', '\\.');
    const notes = changelog.match(new RegExp(`^## ${escaped} - [^\\n]+\\n([\\s\\S]*?)(?=^## |$(?![\\s\\S]))`, 'm'))?.[1]?.trim();
    if (!notes) throw new Error(`Missing changelog entry for ${version}`);
    return notes;
}

export function releaseNotes(version) {
    return extractReleaseNotes(readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8'), version);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    const version = process.argv[2] || JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version;
    console.log(releaseNotes(version));
}
