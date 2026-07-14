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
const seconds = Math.max(3, Number(argValue('--seconds')) || 8);

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

if (!existsSync(executablePath)) {
    console.error(`Packaged app executable not found: ${executablePath}`);
    process.exit(1);
}

const tempHome = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-social-smoke-'));
const outputPath = path.join(tempHome, 'app.log');
const output = await import('node:fs').then((fs) => fs.createWriteStream(outputPath));
const child = spawn(executablePath, [], {
    cwd: projectRoot,
    env: {
        ...process.env,
        HOME: tempHome,
        RUST_BACKTRACE: '1',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
});

child.stdout.pipe(output);
child.stderr.pipe(output);

let exited = false;
let exitCode = null;
child.on('exit', (code) => {
    exited = true;
    exitCode = code;
});

await new Promise((resolve) => setTimeout(resolve, seconds * 1000));

if (exited) {
    const contents = await readFile(outputPath, 'utf8').catch(() => '');
    await rm(tempHome, { recursive: true, force: true });
    console.error(`Packaged app exited before ${seconds}s with status ${exitCode ?? 'unknown'}.`);

    if (contents.trim()) {
        console.error(contents.trim().split('\n').slice(0, 80).join('\n'));
    }

    process.exit(exitCode || 1);
}

child.kill();
await new Promise((resolve) => child.once('exit', resolve));
await rm(tempHome, { recursive: true, force: true });
console.log(`Packaged app stayed running for ${seconds}s.`);
