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

async function waitForReport(reportFile, predicate, deadline) {
    let lastError;

    while (Date.now() < deadline) {
        try {
            const report = JSON.parse(await readFile(reportFile, 'utf8'));

            if (predicate(report)) {
                return report;
            }
        } catch (error) {
            lastError = error;
        }

        await new Promise((resolve) => setTimeout(resolve, 250));
    }

    throw new Error(`Updater smoke did not produce its final report before timeout${lastError ? `: ${lastError}` : '.'}`);
}

if (process.platform !== 'darwin') {
    console.log('Skipping updater smoke test on non-macOS host.');
    process.exit(0);
}

const sourceAppPath = path.resolve(
    projectRoot,
    argValue('--app') || 'src-tauri/target/release/bundle/macos/Dust Wave Social.app',
);
const mode = argValue('--mode') || 'download';
const expectedVersion = argValue('--expected-version')
    || JSON.parse(await readFile(path.join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8')).version;
const timeoutMs = Math.max(30_000, Number(argValue('--timeout-ms')) || 180_000);
const sourcePlistPath = path.join(sourceAppPath, 'Contents', 'Info.plist');

if (!existsSync(sourcePlistPath)) {
    throw new Error(`Packaged app not found: ${sourceAppPath}`);
}

// GitHub's macOS runner can place the Cargo target directory behind a symlink.
// Tauri's updater deliberately rejects a starting executable with symlinked path
// components, so exercise the signed bundle from a canonical staging directory.
const tempBase = existsSync('/private/tmp') ? '/private/tmp' : os.tmpdir();
const tempRoot = await mkdtemp(path.join(tempBase, 'dust-wave-updater-smoke-'));
const appPath = path.join(tempRoot, path.basename(sourceAppPath));
const reportPath = path.join(tempRoot, 'report.json');
const launchReportPath = path.join(tempRoot, 'launch-report.json');
let child;
let launchChild;

try {
    const staged = spawnSync('/usr/bin/ditto', [sourceAppPath, appPath], {
        encoding: 'utf8',
        stdio: 'pipe',
    });

    if (staged.status !== 0) {
        throw new Error(staged.stderr || `Unable to stage packaged app at ${appPath}`);
    }

    const plistPath = path.join(appPath, 'Contents', 'Info.plist');
    const executablePath = path.join(appPath, 'Contents', 'MacOS', plistValue(plistPath, 'CFBundleExecutable'));

    if (!existsSync(executablePath)) {
        throw new Error(`Packaged app executable not found: ${executablePath}`);
    }

    child = spawn(executablePath, [], {
        cwd: tempRoot,
        env: {
            ...process.env,
            DUSTWAVE_SMOKE_REPORT: reportPath,
            DUSTWAVE_UPDATER_EXPECT_VERSION: expectedVersion,
            DUSTWAVE_UPDATER_SMOKE: mode === 'bridge' ? 'install' : mode,
            ...(mode === 'download' ? { DUSTWAVE_UPDATER_SMOKE_FORCE_UPDATE: '1' } : {}),
        },
        stdio: ['ignore', 'pipe', 'pipe'],
    });
    let output = '';
    const deadline = Date.now() + timeoutMs;

    child.stdout.on('data', (chunk) => {
        output += chunk;
    });
    child.stderr.on('data', (chunk) => {
        output += chunk;
    });

    const exitCode = await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            child.kill();
            reject(new Error(`Updater smoke timed out after ${timeoutMs}ms.`));
        }, Math.max(1, deadline - Date.now()));

        child.once('error', (error) => {
            clearTimeout(timeout);
            reject(error);
        });
        child.once('exit', (code) => {
            clearTimeout(timeout);
            resolve(code ?? 1);
        });
    });
    const report = mode === 'hop'
        ? await waitForReport(reportPath, (candidate) => candidate.restarted === true, deadline)
        : JSON.parse(await readFile(reportPath, 'utf8'));

    if (exitCode !== 0 || !report.ok || report.kind !== 'updater' || !report.found_update) {
        throw new Error(`Updater smoke failed: ${JSON.stringify(report)}${output.trim() ? `\n${output.trim()}` : ''}`);
    }

    if (report.update_version !== expectedVersion || !Number.isFinite(report.downloaded_bytes) || report.downloaded_bytes <= 0) {
        throw new Error(`Updater smoke returned invalid release metadata: ${JSON.stringify(report)}`);
    }

    if (mode !== 'download' && !report.install_started) {
        throw new Error(`Updater install smoke did not start installation: ${JSON.stringify(report)}`);
    }

    if (mode === 'hop') {
        if (!report.restart_requested || !report.restarted) {
            throw new Error(`Updater hop did not complete its restart: ${JSON.stringify(report)}`);
        }

        if (!Number.isInteger(report.source_pid) || !Number.isInteger(report.final_pid) || report.source_pid === report.final_pid) {
            throw new Error(`Updater hop did not transition to a new process: ${JSON.stringify(report)}`);
        }

        if (report.running_version !== expectedVersion) {
            throw new Error(`Updater hop relaunched version ${report.running_version}; expected ${expectedVersion}.`);
        }
    }

    if (mode !== 'download') {
        const installedVersion = plistValue(plistPath, 'CFBundleShortVersionString');

        if (installedVersion !== expectedVersion) {
            throw new Error(`Updater install left staged app at ${installedVersion}; expected ${expectedVersion}.`);
        }
    }

    let processProof = mode === 'hop' ? `, PID ${report.source_pid} -> ${report.final_pid}` : '';

    if (mode === 'bridge') {
        const launchEnv = {
            ...process.env,
            DUSTWAVE_DESKTOP_SMOKE: 'launch',
            DUSTWAVE_DESKTOP_SMOKE_REPORT: launchReportPath,
        };
        delete launchEnv.DUSTWAVE_SMOKE_REPORT;
        delete launchEnv.DUSTWAVE_UPDATER_EXPECT_VERSION;
        delete launchEnv.DUSTWAVE_UPDATER_SMOKE;
        delete launchEnv.DUSTWAVE_UPDATER_SMOKE_FORCE_UPDATE;

        launchChild = spawn(executablePath, [], {
            cwd: tempRoot,
            env: launchEnv,
            stdio: ['ignore', 'pipe', 'pipe'],
        });
        let launchOutput = '';

        launchChild.stdout.on('data', (chunk) => {
            launchOutput += chunk;
        });
        launchChild.stderr.on('data', (chunk) => {
            launchOutput += chunk;
        });

        const launchExitCode = await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                launchChild.kill();
                reject(new Error(`Installed-app launch smoke timed out after ${timeoutMs}ms.`));
            }, Math.max(1, deadline - Date.now()));

            launchChild.once('error', (error) => {
                clearTimeout(timeout);
                reject(error);
            });
            launchChild.once('exit', (code) => {
                clearTimeout(timeout);
                resolve(code ?? 1);
            });
        });
        const launchReport = JSON.parse(await readFile(launchReportPath, 'utf8'));

        if (launchExitCode !== 0 || !launchReport.ok || launchReport.kind !== 'launch') {
            throw new Error(`Installed-app launch smoke failed: ${JSON.stringify(launchReport)}${launchOutput.trim() ? `\n${launchOutput.trim()}` : ''}`);
        }

        if (launchReport.package_version !== expectedVersion || child.pid === launchChild.pid) {
            throw new Error(`Installed-app launch did not start ${expectedVersion} in a new process: ${JSON.stringify(launchReport)}`);
        }

        processProof = `, manual bridge PID ${child.pid} -> ${launchChild.pid}`;
    }

    console.log(`Updater ${mode} smoke passed for ${report.package_version} -> ${report.update_version} (${report.downloaded_bytes} bytes${processProof}).`);
} finally {
    if (child && !child.killed && child.exitCode === null) {
        child.kill();
    }
    if (launchChild && !launchChild.killed && launchChild.exitCode === null) {
        launchChild.kill();
    }
    await rm(tempRoot, { recursive: true, force: true });
}
