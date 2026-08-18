#!/usr/bin/env node

import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawn, spawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : null;
}

if (process.platform !== 'darwin') {
    console.log('Skipping packaged app launch smoke test on non-macOS host.');
    process.exit(0);
}

const appPath = path.resolve(
    projectRoot,
    argValue('--app') || 'src-tauri/target/release/bundle/macos/Dust Wave Social.app'
);
const seconds = Math.max(3, Number(argValue('--seconds')) || 15);

if (!existsSync(appPath)) {
    console.error(`Packaged app not found: ${appPath}`);
    process.exit(1);
}

const plistPath = path.join(appPath, 'Contents', 'Info.plist');
const executableNameResult = spawnSync('/usr/libexec/PlistBuddy', ['-c', 'Print :CFBundleExecutable', plistPath], {
    encoding: 'utf8',
    stdio: 'pipe',
});

if (executableNameResult.status !== 0) {
    console.error(executableNameResult.stderr || `Unable to read CFBundleExecutable from ${plistPath}`);
    process.exit(executableNameResult.status || 1);
}

const executableName = String(executableNameResult.stdout || '').trim();
const executablePath = path.join(appPath, 'Contents', 'MacOS', executableName);
const bundleVersionResult = spawnSync('/usr/libexec/PlistBuddy', ['-c', 'Print :CFBundleShortVersionString', plistPath], {
    encoding: 'utf8',
    stdio: 'pipe',
});
const bundleVersion = String(bundleVersionResult.stdout || '').trim();

if (!existsSync(executablePath)) {
    console.error(`Packaged app executable not found: ${executablePath}`);
    process.exit(1);
}

const tempRoot = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-social-smoke-'));
const reportPath = path.join(tempRoot, 'report.json');
const child = spawn(executablePath, [], {
    cwd: projectRoot,
    env: {
        ...process.env,
        DUSTWAVE_DESKTOP_SMOKE: 'launch',
        DUSTWAVE_DESKTOP_SMOKE_DELAY_MS: '1500',
        DUSTWAVE_DESKTOP_SMOKE_REPORT: reportPath,
        RUST_BACKTRACE: '1',
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
            reject(new Error(`Packaged app launch smoke timed out after ${seconds}s.`));
        }, seconds * 1000);

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

    if (exitCode !== 0 || !report.ok || report.kind !== 'launch' || report.package_version !== bundleVersion) {
        throw new Error(`Packaged app launch smoke failed: ${JSON.stringify(report)}${output.trim() ? `\n${output.trim()}` : ''}`);
    }

    console.log(`Packaged app launch smoke passed for ${report.package_version}.`);
} finally {
    if (!child.killed && child.exitCode === null) {
        child.kill();
    }
    await rm(tempRoot, { recursive: true, force: true });
}
