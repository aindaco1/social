#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);

function argValue(name) {
    const index = args.indexOf(name);

    return index >= 0 ? args[index + 1] : null;
}

const appPath = path.resolve(
    projectRoot,
    argValue('--app') || 'src-tauri/target/release/bundle/macos/Dust Wave Social.app',
);
const dmgPath = path.resolve(
    projectRoot,
    argValue('--dmg') || 'src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg',
);
const latestJsonPath = path.resolve(
    projectRoot,
    argValue('--latest-json') || 'src-tauri/target/release/bundle/latest.json',
);
const requireStapled = args.includes('--require-stapled');
const requireUpdater = args.includes('--require-updater');
const allowMissingMedia = args.includes('--allow-missing-media');
const failures = [];
const warnings = [];

function relative(filePath) {
    return path.relative(projectRoot, filePath);
}

function run(command, commandArgs, options = {}) {
    const result = spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: 'pipe',
        shell: false,
    });
    const status = result.status ?? 1;
    const output = `${result.stdout || ''}${result.stderr || ''}${result.error?.message || ''}`.trim();

    if (status !== 0 && !options.allowFailure) {
        fail(`${command} ${commandArgs.join(' ')} failed${output ? `:\n${output}` : ''}`);
    }

    return { status, output };
}

function ok(label, detail = '') {
    console.log(`[ok] ${label}${detail ? ` - ${detail}` : ''}`);
}

function warn(label, detail = '') {
    warnings.push({ label, detail });
    console.log(`[warn] ${label}${detail ? ` - ${detail}` : ''}`);
}

function fail(message) {
    failures.push(message);
    console.error(`[fail] ${message}`);
}

function requirePath(label, filePath) {
    if (!existsSync(filePath)) {
        fail(`${label} missing at ${relative(filePath)}`);

        return false;
    }

    ok(label, `${relative(filePath)} (${formatBytes(statSync(filePath).size)})`);

    return true;
}

function formatBytes(bytes) {
    if (bytes < 1024) {
        return `${bytes} B`;
    }

    const units = ['KB', 'MB', 'GB'];
    let value = bytes / 1024;
    let unit = units.shift();

    while (value >= 1024 && units.length) {
        value /= 1024;
        unit = units.shift();
    }

    return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`;
}

function sha256(filePath) {
    return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function plistValue(plistPath, key) {
    const result = run('/usr/libexec/PlistBuddy', ['-c', `Print :${key}`, plistPath], {
        allowFailure: true,
    });

    return result.status === 0 ? result.output.trim() : '';
}

function checkMachOArm64(label, filePath) {
    const result = run('file', [filePath], { allowFailure: true });

    if (result.status !== 0 || !result.output.includes('Mach-O 64-bit') || !result.output.includes('arm64')) {
        fail(`${label} is not an arm64 Mach-O executable: ${result.output || relative(filePath)}`);

        return;
    }

    ok(`${label} architecture`, 'arm64');
}

function checkCodeSignature(label, filePath) {
    run('codesign', ['--verify', '--strict', '--verbose=2', filePath]);
    ok(`${label} code signature`, 'valid on disk');
}

if (process.platform !== 'darwin') {
    fail('macOS release artifact verification must run on macOS.');
}

if (requirePath('macOS app bundle', appPath)) {
    const plistPath = path.join(appPath, 'Contents', 'Info.plist');
    const executableName = plistValue(plistPath, 'CFBundleExecutable');
    const bundleIdentifier = plistValue(plistPath, 'CFBundleIdentifier');
    const version = plistValue(plistPath, 'CFBundleShortVersionString');
    const executablePath = path.join(appPath, 'Contents', 'MacOS', executableName);

    ok('bundle identity', `${bundleIdentifier || 'unknown'} ${version || 'unknown'}`);

    if (requirePath('main executable', executablePath)) {
        checkMachOArm64('main executable', executablePath);
    }

    run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', appPath]);
    ok('deep app code signature', 'valid on disk');

    const codeDetails = run('codesign', ['-dv', '--verbose=4', appPath]).output;

    if (/flags=.*\bruntime\b/.test(codeDetails)) {
        ok('hardened runtime', 'enabled');
    } else {
        fail('hardened runtime flag is missing from the app signature');
    }

    const authority = codeDetails.match(/Authority=(Developer ID Application:[^\n]+)/)?.[1] || '';
    const teamId = codeDetails.match(/TeamIdentifier=([A-Z0-9]+)/)?.[1] || '';

    if (authority) {
        ok('Developer ID authority', authority);
    } else {
        fail('Developer ID Application authority is missing from the app signature');
    }

    if (teamId) {
        ok('TeamIdentifier', teamId);
    }

    for (const sidecar of ['ffmpeg', 'ffprobe']) {
        const sidecarPath = path.join(appPath, 'Contents', 'MacOS', sidecar);

        if (!existsSync(sidecarPath)) {
            if (allowMissingMedia) {
                warn(`${sidecar} sidecar`, 'not bundled');
            } else {
                fail(`${sidecar} sidecar missing from app bundle`);
            }

            continue;
        }

        ok(`${sidecar} sidecar`, `${relative(sidecarPath)} (${formatBytes(statSync(sidecarPath).size)})`);
        checkMachOArm64(`${sidecar} sidecar`, sidecarPath);
        checkCodeSignature(`${sidecar} sidecar`, sidecarPath);
    }
}

if (requirePath('release DMG', dmgPath)) {
    run('hdiutil', ['verify', dmgPath]);
    ok('DMG checksum', 'valid');
    ok('DMG sha256', sha256(dmgPath));

    const stapler = run('xcrun', ['stapler', 'validate', dmgPath], { allowFailure: true });

    if (stapler.status === 0) {
        ok('notarization ticket', 'stapled');
    } else if (requireStapled) {
        fail(`notarization ticket is not stapled:\n${stapler.output}`);
    } else {
        warn('notarization ticket', 'not stapled yet');
    }

    if (requireStapled) {
        run('spctl', ['-a', '-vv', dmgPath]);
        ok('Gatekeeper assessment', 'accepted');
    }
}

const updaterArtifactPath = path.resolve(projectRoot, 'src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz');
const updaterSignaturePath = path.resolve(projectRoot, `${updaterArtifactPath}.sig`);
const hasUpdaterArtifacts = existsSync(latestJsonPath) || existsSync(updaterArtifactPath) || existsSync(updaterSignaturePath);

if (requireUpdater || hasUpdaterArtifacts) {
    const hasLatest = requirePath('updater latest.json', latestJsonPath);
    const hasArchive = requirePath('updater app archive', updaterArtifactPath);
    const hasSignature = requirePath('updater app signature', updaterSignaturePath);

    if (hasLatest) {
        let latest;

        try {
            latest = JSON.parse(readFileSync(latestJsonPath, 'utf8'));
        } catch (error) {
            fail(`latest.json is not valid JSON: ${error.message}`);
            latest = {};
        }

        const platform = latest.platforms?.['darwin-aarch64'];

        if (!latest.version) {
            fail('latest.json is missing version');
        }

        if (!platform?.url || !platform?.signature) {
            fail('latest.json is missing darwin-aarch64 url or signature');
        } else {
            ok('updater latest.json', `${latest.version} -> ${platform.url}`);
        }
    }

    if (hasArchive) {
        ok('updater archive sha256', sha256(updaterArtifactPath));
    }

    if (hasSignature) {
        ok('updater signature', 'present');
    }
}

if (warnings.length) {
    console.log(`\nRelease artifact verification completed with ${warnings.length} warning(s).`);
}

if (failures.length) {
    console.error(`\nRelease artifact verification failed with ${failures.length} issue(s).`);
    process.exit(1);
}

if (!warnings.length) {
    console.log('\nRelease artifact verification completed without warnings.');
}
