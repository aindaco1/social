import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { signLocalMacosCode } from './macos-local-signing.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));

export const projectRoot = path.resolve(scriptDirectory, '..', '..');
export const mediaStagingConfigPath = path.join(
    projectRoot,
    'workers',
    'media-staging',
    'wrangler.generated.jsonc',
);
export const keychainService = 'com.dustwave.social';
export const keychainAccount = 'services/media_staging/client_secret';
export const defaultMediaStagingUrl = 'https://dustwave-media-staging.jogo.workers.dev';

export function run(command, args, options = {}) {
    return spawnSync(command, args, {
        cwd: options.cwd || projectRoot,
        encoding: 'utf8',
        shell: false,
        input: options.input,
        stdio: options.input === undefined ? 'pipe' : ['pipe', 'pipe', 'pipe'],
    });
}

export function requireSuccess(result, description) {
    if (result.status === 0) {
        return result;
    }

    const detail = String(result.stderr || result.stdout || '').trim();
    throw new Error(`${description} failed${detail ? `: ${detail}` : ''}`);
}

export function cloudflareSecretNames() {
    const result = requireSuccess(run('npx', [
        'wrangler',
        'secret',
        'list',
        '--config',
        mediaStagingConfigPath,
        '--format',
        'json',
    ]), 'Listing Cloudflare Worker secrets');

    return new Set(
        JSON.parse(result.stdout || '[]')
            .map((item) => String(item.name || '').trim())
            .filter(Boolean),
    );
}

function withSignedKeychainHelper(callback) {
    const temporaryDirectory = mkdtempSync(path.join(tmpdir(), 'dust-wave-keychain-'));
    const helperPath = path.join(temporaryDirectory, 'keychain-helper');
    const sourcePath = path.join(projectRoot, 'scripts', 'set-macos-keychain-secret.swift');

    try {
        requireSuccess(run('/usr/bin/xcrun', [
            'swiftc',
            '-framework', 'Security',
            '-o', helperPath,
            sourcePath,
        ]), 'Building the signed Keychain helper');
        const signing = signLocalMacosCode(helperPath);

        if (!signing.stable) {
            throw new Error('A Developer ID Application identity is required to provision stable Keychain access.');
        }

        return callback(helperPath);
    } finally {
        rmSync(temporaryDirectory, { recursive: true, force: true });
    }
}

export function storeLocalMediaStagingToken(token) {
    withSignedKeychainHelper((helperPath) => {
        requireSuccess(run(helperPath, [
            'set',
            keychainService,
            keychainAccount,
            'Dust Wave Social - Instagram Local Media',
        ], { input: `${token}\n` }), 'Saving the Instagram Local Media credential in Keychain');
    });
}

export function readLocalMediaStagingToken() {
    const token = withSignedKeychainHelper((helperPath) => {
        const result = requireSuccess(run(helperPath, [
            'get',
            keychainService,
            keychainAccount,
        ]), 'Reading the Instagram Local Media credential from Keychain');

        return String(result.stdout || '').trim();
    });

    if (!token) {
        throw new Error('The Instagram Local Media Keychain credential is empty.');
    }

    return token;
}
