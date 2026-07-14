import { spawnSync } from 'node:child_process';

export function normalizeGitHubRepoSlug(value) {
    const trimmed = String(value || '').trim();

    if (/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(trimmed)) {
        return trimmed;
    }

    const sshMatch = trimmed.match(/^git@github\.com:([^/]+)\/(.+?)(?:\.git)?$/);

    if (sshMatch) {
        return `${sshMatch[1]}/${sshMatch[2]}`;
    }

    const httpsMatch = trimmed.match(/^https:\/\/github\.com\/([^/]+)\/(.+?)(?:\.git)?\/?$/);

    if (httpsMatch) {
        return `${httpsMatch[1]}/${httpsMatch[2]}`;
    }

    return '';
}

export function gitRemoteRepoSlug(projectRoot, remoteName = 'origin') {
    const result = spawnSync('git', ['remote', 'get-url', remoteName], {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });

    if (result.status !== 0) {
        return '';
    }

    return normalizeGitHubRepoSlug(result.stdout);
}
