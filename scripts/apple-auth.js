import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

export const DEFAULT_APPLE_AUTH_DIR = path.join(
    os.homedir(),
    'Library',
    'Mobile Documents',
    'com~apple~CloudDocs',
    'Apple Auth',
);

export function appleAuthDirectory() {
    return path.resolve(process.env.DUSTWAVE_APPLE_AUTH_DIR || DEFAULT_APPLE_AUTH_DIR);
}

export function authPath(...parts) {
    return path.join(appleAuthDirectory(), ...parts);
}

export function fileExists(filePath) {
    return existsSync(filePath);
}

export async function readTrimmedFile(filePath) {
    return (await readFile(filePath, 'utf8')).trim();
}

export function appleApiKeyIdFromPath(filePath) {
    const match = path.basename(filePath).match(/^AuthKey_([A-Z0-9]+)\.p8$/);

    return match?.[1] || '';
}

export function redactPath(filePath) {
    const home = os.homedir();

    return filePath.startsWith(home) ? filePath.replace(home, '~') : filePath;
}
