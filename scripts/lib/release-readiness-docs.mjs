import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

export const LOCAL_RELEASE_READINESS_PATH = 'artifacts/release-readiness.md';
export const RELEASE_OPERATIONS_PATH = 'docs/RELEASE_OPERATIONS.md';

// Both preflight tools use the same durable-docs contract. Local artifact presence
// is checked separately and must not turn a stale committed snapshot into proof.
export function releaseReadinessDocumentationReady(projectRoot) {
    const read = (relativePath) => {
        const filePath = path.join(projectRoot, relativePath);
        return existsSync(filePath) ? readFileSync(filePath, 'utf8') : '';
    };
    const generator = read('scripts/generate-mvp-release-notes.mjs');
    const runbook = read(RELEASE_OPERATIONS_PATH);

    return generator.includes("argValue('--output') || LOCAL_RELEASE_READINESS_PATH")
        && generator.includes('MVP_RELEASE_NOTES_START')
        && runbook.includes(LOCAL_RELEASE_READINESS_PATH)
        && runbook.includes('## Rollback Plan')
        && !runbook.includes('<!-- MVP_RELEASE_NOTES_START -->')
        && read('.gitignore').split(/\r?\n/).includes('/artifacts/');
}
