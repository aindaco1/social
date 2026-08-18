#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { spawn, spawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : '';
}

function plistValue(plistPath, key) {
    const result = spawnSync('/usr/libexec/PlistBuddy', ['-c', `Print :${key}`, plistPath], {
        encoding: 'utf8',
        stdio: 'pipe',
    });

    if (result.status !== 0) {
        throw new Error(result.stderr || `Unable to read ${key} from ${plistPath}`);
    }

    return String(result.stdout || '').trim();
}

if (process.platform !== 'darwin') {
    console.log('Skipping updater smoke test on non-macOS host.');
    process.exit(0);
}

const appPath = path.resolve(
    projectRoot,
    argValue('--app') || 'src-tauri/target/release/bundle/macos/Dust Wave Social.app',
);
const mode = argValue('--mode') || 'download';
const expectedVersion = argValue('--expected-version')
    || JSON.parse(await readFile(path.join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8')).version;
const timeoutMs = Math.max(30_000, Number(argValue('--timeout-ms')) || 180_000);
const plistPath = path.join(appPath, 'Contents', 'Info.plist');

if (!existsSync(plistPath)) {
    throw new Error(`Packaged app not found: ${appPath}`);
}

const executablePath = path.join(appPath, 'Contents', 'MacOS', plistValue(plistPath, 'CFBundleExecutable'));

if (!existsSync(executablePath)) {
    throw new Error(`Packaged app executable not found: ${executablePath}`);
}

const tempRoot = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-updater-smoke-'));
const reportPath = path.join(tempRoot, 'report.json');
const child = spawn(executablePath, [], {
    cwd: projectRoot,
    env: {
        ...process.env,
        DUSTWAVE_SMOKE_REPORT: reportPath,
        DUSTWAVE_UPDATER_EXPECT_VERSION: expectedVersion,
        DUSTWAVE_UPDATER_SMOKE: mode,
        ...(mode === 'download' ? { DUSTWAVE_UPDATER_SMOKE_FORCE_UPDATE: '1' } : {}),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
});
let output = '';

child.stdout.on('data', (chunk) => {
    output += chunk;
});
child.stderr.on('data', (chunk) => {
    output += chunk;
});

try {
    const exitCode = await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            child.kill();
            reject(new Error(`Updater smoke timed out after ${timeoutMs}ms.`));
        }, timeoutMs);

        child.once('error', (error) => {
            clearTimeout(timeout);
            reject(error);
        });
        child.once('exit', (code) => {
            clearTimeout(timeout);
            resolve(code ?? 1);
        });
    });
    const report = JSON.parse(await readFile(reportPath, 'utf8'));

    if (exitCode !== 0 || !report.ok || report.kind !== 'updater' || !report.found_update) {
        throw new Error(`Updater smoke failed: ${JSON.stringify(report)}${output.trim() ? `\n${output.trim()}` : ''}`);
    }

    if (report.update_version !== expectedVersion || !Number.isFinite(report.downloaded_bytes) || report.downloaded_bytes <= 0) {
        throw new Error(`Updater smoke returned invalid release metadata: ${JSON.stringify(report)}`);
    }

    if (mode !== 'download' && !report.install_started) {
        throw new Error(`Updater install smoke did not start installation: ${JSON.stringify(report)}`);
    }

    console.log(`Updater ${mode} smoke passed for ${report.package_version} -> ${report.update_version} (${report.downloaded_bytes} bytes).`);
} finally {
    if (!child.killed && child.exitCode === null) {
        child.kill();
    }
    await rm(tempRoot, { recursive: true, force: true });
}
