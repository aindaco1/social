#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { readdir, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
    appleApiKeyIdFromPath,
    appleAuthDirectory,
    authPath,
    fileExists,
    readTrimmedFile,
    redactPath,
} from './apple-auth.js';
import { gitRemoteRepoSlug } from './release-repo.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const withMedia = args.includes('--media');
const withUpdater = args.includes('--updater');
const withNotarization = args.includes('--notarize');
const withDmg = args.includes('--dmg');
const importCertificate = args.includes('--import-certificate');
const skipTemporaryKeychain = args.includes('--no-temp-keychain');
const passThroughArgs = args.filter(
    (arg) => !['--media', '--updater', '--notarize', '--dmg', '--import-certificate', '--no-temp-keychain'].includes(arg),
);
const signingConfigPath = path.join(
    projectRoot,
    'src-tauri',
    'tauri.macos-signing.generated.conf.json',
);

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: options.stdio || 'pipe',
        env: options.env || process.env,
    });
}

function keychainList() {
    const result = run('security', ['list-keychains', '-d', 'user']);

    return String(result.stdout || '')
        .split('\n')
        .map((line) => line.trim().replace(/^"|"$/g, ''))
        .filter(Boolean);
}

function defaultKeychain() {
    const result = run('security', ['default-keychain', '-d', 'user']);

    return String(result.stdout || '').trim().replace(/^"|"$/g, '');
}

function codesignIdentity() {
    const configured = String(process.env.APPLE_SIGNING_IDENTITY || '').trim();

    if (configured) {
        return configured;
    }

    const result = run('security', ['find-identity', '-p', 'codesigning', '-v']);
    const output = `${result.stdout || ''}\n${result.stderr || ''}`;
    const match = output.match(/"((?:Developer ID Application:)[^"]+)"/);

    return match?.[1] || '';
}

function teamIdFromIdentity(identity) {
    return identity.match(/\(([A-Z0-9]{10})\)$/)?.[1] || '';
}

async function importDeveloperIdCertificateIfNeeded() {
    if (codesignIdentity() && !importCertificate) {
        return;
    }

    const certificatePath = authPath('developer-id-application.p12');
    const passwordPath = authPath('apple-p12-password.txt');

    if (!fileExists(certificatePath) || !fileExists(passwordPath)) {
        return;
    }

    const password = await readTrimmedFile(passwordPath);
    const result = run('security', [
        'import',
        certificatePath,
        '-P',
        password,
        '-A',
        '-T',
        '/usr/bin/codesign',
        '-T',
        '/usr/bin/productsign',
    ]);

    if (result.status !== 0 && !String(result.stderr || '').includes('already exists')) {
            throw new Error(result.stderr || result.stdout || 'Failed to import Developer ID certificate.');
    }
}

async function prepareTemporarySigningKeychain() {
    if (skipTemporaryKeychain || process.platform !== 'darwin') {
        return () => {};
    }

    const certificateFromEnv = String(process.env.APPLE_CERTIFICATE || '').trim();
    let certificatePath = authPath('developer-id-application.p12');
    let certificatePassword = String(process.env.APPLE_CERTIFICATE_PASSWORD || '').trim();
    let removeTemporaryCertificate = async () => {};

    if (certificateFromEnv) {
        certificatePath = path.join(os.tmpdir(), `dustwave-certificate-${process.pid}-${Date.now()}.p12`);
        await writeFile(certificatePath, Buffer.from(certificateFromEnv, 'base64'));
        removeTemporaryCertificate = async () => {
            await rm(certificatePath, { force: true });
        };
    }

    const passwordPath = authPath('apple-p12-password.txt');

    if (!certificatePassword && fileExists(passwordPath)) {
        certificatePassword = await readTrimmedFile(passwordPath);
    }

    if (!fileExists(certificatePath) || !certificatePassword) {
        return () => {};
    }

    const originalKeychains = keychainList();
    const originalDefaultKeychain = defaultKeychain();
    const keychainPath = path.join(
        os.tmpdir(),
        `dustwave-signing-${process.pid}-${Date.now()}.keychain-db`,
    );
    const keychainPassword = `dustwave-${process.pid}-${Date.now()}`;
    for (const [command, commandArgs] of [
        ['security', ['create-keychain', '-p', keychainPassword, keychainPath]],
        ['security', ['unlock-keychain', '-p', keychainPassword, keychainPath]],
        ['security', ['set-keychain-settings', '-lut', '21600', keychainPath]],
        ['security', ['import', certificatePath, '-k', keychainPath, '-P', certificatePassword, '-A', '-T', '/usr/bin/codesign', '-T', '/usr/bin/productsign']],
        ['security', ['set-key-partition-list', '-S', 'apple-tool:,apple:', '-s', '-k', keychainPassword, keychainPath]],
        ['security', ['list-keychains', '-d', 'user', '-s', keychainPath, ...originalKeychains]],
        ['security', ['default-keychain', '-d', 'user', '-s', keychainPath]],
    ]) {
        const result = run(command, commandArgs);

        if (result.status !== 0) {
            console.error(result.stderr || result.stdout || `Failed to run ${command}.`);
            process.exit(result.status || 1);
        }
    }

    console.log(`Using temporary signing keychain: ${redactPath(keychainPath)}`);

    return async () => {
        if (originalKeychains.length) {
            run('security', ['list-keychains', '-d', 'user', '-s', ...originalKeychains]);
        }

        if (originalDefaultKeychain) {
            run('security', ['default-keychain', '-d', 'user', '-s', originalDefaultKeychain]);
        }

        run('security', ['delete-keychain', keychainPath]);
        await removeTemporaryCertificate();
    };
}

async function firstAppleApiKeyPath() {
    const directory = appleAuthDirectory();

    if (!fileExists(directory)) {
        return '';
    }

    const entries = await readdir(directory);
    const key = entries.find((entry) => /^AuthKey_[A-Z0-9]+\.p8$/.test(entry));

    return key ? path.join(directory, key) : '';
}

async function optionalIssuer() {
    for (const fileName of [
        'apple-api-issuer.txt',
        'app-store-connect-issuer.txt',
        'issuer.txt',
    ]) {
        const filePath = authPath(fileName);

        if (fileExists(filePath)) {
            return readTrimmedFile(filePath);
        }
    }

    return String(process.env.APPLE_API_ISSUER || '').trim();
}

async function writeSigningConfig(identity, teamId) {
    const config = {
        bundle: {
            macOS: {
                signingIdentity: identity,
                hardenedRuntime: true,
            },
        },
    };
    const providerShortName = String(process.env.APPLE_PROVIDER_SHORT_NAME || teamId || '').trim();

    if (providerShortName) {
        config.bundle.macOS.providerShortName = providerShortName;
    }

    await writeFile(signingConfigPath, `${JSON.stringify(config, null, 2)}\n`);
}

async function buildEnvironment(identity, teamId) {
    const env = {
        ...process.env,
        APPLE_SIGNING_IDENTITY: identity,
    };

    if (teamId) {
        env.APPLE_TEAM_ID = env.APPLE_TEAM_ID || teamId;
        env.APPLE_PROVIDER_SHORT_NAME = env.APPLE_PROVIDER_SHORT_NAME || teamId;
    }

    if (!withNotarization) {
        delete env.APPLE_API_KEY;
        delete env.APPLE_API_KEY_PATH;
        delete env.APPLE_API_ISSUER;
        delete env.APPLE_ID;
        delete env.APPLE_PASSWORD;
    }

    if (withNotarization) {
        const apiKeyPath = String(process.env.APPLE_API_KEY_PATH || '').trim() || (await firstAppleApiKeyPath());
        const apiKey = String(process.env.APPLE_API_KEY || '').trim() || appleApiKeyIdFromPath(apiKeyPath);
        const issuer = await optionalIssuer();

        if (!apiKeyPath || !apiKey) {
            throw new Error(
                `Notarization requested, but no AuthKey_*.p8 file was found in ${redactPath(appleAuthDirectory())}.`,
            );
        }

        env.APPLE_API_KEY_PATH = apiKeyPath;
        env.APPLE_API_KEY = apiKey;

        if (issuer) {
            env.APPLE_API_ISSUER = issuer;
        } else {
            console.warn('No Apple API issuer file/env was found; trying Individual API key mode.');
        }
    }

    if (withUpdater) {
        const privateKeyPath = authPath('tauri-updater-private.key');
        const passwordPath = authPath('tauri-updater-password.txt');
        const publicKeyPath = authPath('tauri-updater-public-key.txt');
        const privateKeyFromEnv = String(env.TAURI_SIGNING_PRIVATE_KEY || '').trim();
        const privateKeyPathFromEnv = String(env.TAURI_SIGNING_PRIVATE_KEY_PATH || '').trim();
        const passwordFromEnv = String(env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD || '').trim();
        const publicKey = String(env.TAURI_UPDATER_PUBLIC_KEY || env.DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY || '').trim()
            || (fileExists(publicKeyPath) ? await readTrimmedFile(publicKeyPath) : '');
        const hasPrivateKey = Boolean(privateKeyFromEnv)
            || (Boolean(privateKeyPathFromEnv) && fileExists(privateKeyPathFromEnv))
            || fileExists(privateKeyPath);
        const password = passwordFromEnv || (fileExists(passwordPath) ? await readTrimmedFile(passwordPath) : '');

        if (!hasPrivateKey || !password || !publicKey) {
            throw new Error('Updater artifacts requested, but updater signing values are missing. Run npm run desktop:updater:keys locally or configure CI updater secrets.');
        }

        const releaseRepository = String(env.DUSTWAVE_RELEASE_REPO || env.GITHUB_REPOSITORY || gitRemoteRepoSlug(projectRoot) || '').trim();

        if (!releaseRepository) {
            throw new Error('Updater artifacts requested, but DUSTWAVE_RELEASE_REPO=owner/repo is not set.');
        }

        env.DUSTWAVE_RELEASE_REPO = env.DUSTWAVE_RELEASE_REPO || releaseRepository;
        if (!privateKeyFromEnv && !privateKeyPathFromEnv) {
            env.TAURI_SIGNING_PRIVATE_KEY = await readFile(privateKeyPath, 'utf8');
            env.TAURI_SIGNING_PRIVATE_KEY_PATH = privateKeyPath;
        }
        env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD = password;
        env.TAURI_UPDATER_PUBLIC_KEY = env.TAURI_UPDATER_PUBLIC_KEY || publicKey;
        env.DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY = env.DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY || publicKey;
    }

    return env;
}

let cleanupTemporaryKeychain = () => {};
let exitCode = 0;

try {
    cleanupTemporaryKeychain = await prepareTemporarySigningKeychain();
    await importDeveloperIdCertificateIfNeeded();

    const identity = codesignIdentity();

    if (!identity) {
        throw new Error('No Developer ID Application signing identity is available.');
    }

    const teamId = teamIdFromIdentity(identity);
    await writeSigningConfig(identity, teamId);

    const env = await buildEnvironment(identity, teamId);
    const buildArgs = [
        path.join(scriptDirectory, 'build-desktop-release.mjs'),
        ...(withMedia ? ['--media'] : []),
        ...(withUpdater ? ['--updater'] : []),
        '--config',
        signingConfigPath,
        '--ci',
        ...passThroughArgs,
    ];

    console.log(`Using signing identity: ${identity}`);
    console.log(`Using signing overlay: ${redactPath(signingConfigPath)}`);
    if (withNotarization) {
        console.log('Notarization: enabled');
    }
    if (withUpdater) {
        console.log(`Updater repository: ${env.DUSTWAVE_RELEASE_REPO || env.GITHUB_REPOSITORY}`);
    }

    const build = run(process.execPath, buildArgs, { env, stdio: 'inherit' });
    exitCode = build.status || 0;

    if (exitCode === 0 && withUpdater) {
        const latest = run(process.execPath, [path.join(scriptDirectory, 'generate-tauri-latest-json.mjs')], {
            env,
            stdio: 'inherit',
        });
        exitCode = latest.status || 0;
    }

    if (exitCode === 0 && withDmg) {
        const dmg = run(process.execPath, [path.join(scriptDirectory, 'create-macos-dmg.mjs')], {
            env,
            stdio: 'inherit',
        });
        exitCode = dmg.status || 0;
    }
} catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    exitCode = 1;
} finally {
    await cleanupTemporaryKeychain();
}

if (exitCode !== 0) {
    process.exit(exitCode);
}
