#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const excludedDirectories = new Set(['.git', 'dist', 'node_modules', 'target', 'vendor']);
const headingExemptions = new Set(['LICENSE.md']);
const requiredDocuments = [
    'CHANGELOG.md',
    'LICENSE.md',
    'README.md',
    'SECURITY.md',
    'THIRD_PARTY_NOTICES.md',
    'docs/ARCHITECTURE.md',
    'docs/BEST_PRACTICES.md',
    'docs/FEATURES.md',
    'docs/GIF_PROVIDER_DECISION.md',
    'docs/LOCAL_AI.md',
    'docs/MVP_LAUNCH_PLAN.md',
    'docs/SUPPORT_RUNBOOK.md',
    'docs/USER_FLOWS.md',
    'src-tauri/binaries/README.md',
    'workers/media-staging/README.md',
    'workers/tiktok-broker/README.md',
];
const retiredDocuments = [
    'DUSTWAVE_FINISH_LINE_CHECKLIST.md',
    'DUSTWAVE_MIGRATION.md',
    'MIXPOST_PARITY_AUDIT.md',
    'RELEASE.md',
    'docs/LITERT_MVP_EVALUATION.md',
];
const errors = [];

function relative(filePath) {
    return path.relative(projectRoot, filePath);
}

function markdownFiles(directory) {
    const files = [];

    for (const entry of readdirSync(directory)) {
        if (excludedDirectories.has(entry)) {
            continue;
        }

        const filePath = path.join(directory, entry);
        const metadata = statSync(filePath);

        if (metadata.isDirectory()) {
            files.push(...markdownFiles(filePath));
        } else if (entry.endsWith('.md')) {
            files.push(filePath);
        }
    }

    return files;
}

for (const document of requiredDocuments) {
    if (!existsSync(path.join(projectRoot, document))) {
        errors.push(`Required document is missing: ${document}`);
    }
}

for (const document of retiredDocuments) {
    if (existsSync(path.join(projectRoot, document))) {
        errors.push(`Retired document was reintroduced: ${document}`);
    }
}

const files = markdownFiles(projectRoot);
const retiredPattern = new RegExp(retiredDocuments
    .map((document) => document.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .join('|'));

for (const filePath of files) {
    const source = readFileSync(filePath, 'utf8');
    const headings = source.match(/^# /gm) || [];

    if (!headingExemptions.has(relative(filePath)) && headings.length !== 1) {
        errors.push(`${relative(filePath)} must contain exactly one level-one heading; found ${headings.length}`);
    }

    if (retiredPattern.test(source)) {
        errors.push(`${relative(filePath)} references a retired document`);
    }

    for (const match of source.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g)) {
        let target = match[1].trim().replace(/^<|>$/g, '').split('#')[0];

        if (!target || /^(https?:|mailto:)/i.test(target)) {
            continue;
        }

        try {
            target = decodeURIComponent(target);
        } catch {
            errors.push(`${relative(filePath)} contains an invalid encoded link: ${match[1]}`);
            continue;
        }

        if (!existsSync(path.resolve(path.dirname(filePath), target))) {
            errors.push(`${relative(filePath)} contains a missing relative link: ${match[1]}`);
        }
    }
}

if (errors.length > 0) {
    console.error(errors.map((error) => `- ${error}`).join('\n'));
    process.exit(1);
}

console.log(`Documentation check passed for ${files.length} Markdown files.`);
