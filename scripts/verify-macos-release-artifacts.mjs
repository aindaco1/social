#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
    verifyMacosDmgTrust,
    withMountedMacosDmg,
} from './lib/macos-dmg.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const tauriConfig = JSON.parse(readFileSync(path.join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const expectedVersion = String(tauriConfig.version || '');
const expectedBundleIdentifier = String(tauriConfig.identifier || '');
const defaultDmgName = `Dust Wave Social_${expectedVersion}_${process.arch === 'arm64' ? 'aarch64' : process.arch}.dmg`;

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
    argValue('--dmg') || path.join('src-tauri', 'target', 'release', 'bundle', 'dmg', defaultDmgName),
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
let looseAppIdentity = null;

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

function appIdentity(filePath) {
    const plistPath = path.join(filePath, 'Contents', 'Info.plist');
    const designatedRequirementOutput = run('codesign', ['-dr', '-', filePath], { allowFailure: true }).output;
    const designatedRequirement = designatedRequirementOutput
        .split(/\r?\n/)
        .find((line) => /^designated\s*=>\s*/.test(line))
        ?.replace(/^designated\s*=>\s*/, '')
        .trim() || '';

    return {
        bundleIdentifier: plistValue(plistPath, 'CFBundleIdentifier'),
        version: plistValue(plistPath, 'CFBundleShortVersionString'),
        designatedRequirement,
    };
}

function compareAppIdentity(label, identity) {
    if (!looseAppIdentity) {
        fail(`${label} could not be compared because the loose app identity is unavailable`);
        return false;
    }

    let matches = true;

    for (const field of ['bundleIdentifier', 'version', 'designatedRequirement']) {
        if (!identity[field] || identity[field] !== looseAppIdentity[field]) {
            fail(`${label} ${field} does not match the loose app bundle`);
            matches = false;
        }
    }

    return matches;
}

function gatekeeperDmgArgs(filePath) {
    return ['-a', '-vv', '-t', 'open', '--context', 'context:primary-signature', filePath];
}

if (process.platform !== 'darwin') {
    fail('macOS release artifact verification must run on macOS.');
}

const localAiModelCheck = run(process.execPath, [path.join(scriptDirectory, 'verify-local-ai-models.mjs')], {
    allowFailure: true,
});

if (localAiModelCheck.status === 0) {
    ok('local AI model bundle', localAiModelCheck.output);
} else {
    fail(`local AI model bundle validation failed:\n${localAiModelCheck.output}`);
}

if (requirePath('macOS app bundle', appPath)) {
    const plistPath = path.join(appPath, 'Contents', 'Info.plist');
    const executableName = plistValue(plistPath, 'CFBundleExecutable');
    const bundleIdentifier = plistValue(plistPath, 'CFBundleIdentifier');
    const version = plistValue(plistPath, 'CFBundleShortVersionString');
    const executablePath = path.join(appPath, 'Contents', 'MacOS', executableName);

    ok('bundle identity', `${bundleIdentifier || 'unknown'} ${version || 'unknown'}`);

    if (bundleIdentifier !== expectedBundleIdentifier) {
        fail(`bundle identifier ${bundleIdentifier || '(missing)'} does not match ${expectedBundleIdentifier}`);
    }

    if (version !== expectedVersion) {
        fail(`bundle version ${version || '(missing)'} does not match ${expectedVersion}`);
    }

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

    looseAppIdentity = appIdentity(appPath);

    if (!looseAppIdentity.designatedRequirement) {
        fail('loose app designated requirement is missing');
    }

    if (requireStapled) {
        const appStapler = run('xcrun', ['stapler', 'validate', appPath], { allowFailure: true });
        const appGatekeeper = run('spctl', ['-a', '-vv', '-t', 'execute', appPath], { allowFailure: true });

        if (appStapler.status === 0 && appGatekeeper.status === 0) {
            ok('app notarization and Gatekeeper', 'accepted');
        } else {
            fail(`app notarization or Gatekeeper validation failed:\n${appStapler.output}\n${appGatekeeper.output}`);
        }
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
    try {
        const verification = await withMountedMacosDmg(dmgPath, async ({ appPath: mountedAppPath }) => {
            run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', mountedAppPath]);
            const identityMatches = compareAppIdentity('mounted DMG app', appIdentity(mountedAppPath));

            if (requireStapled) {
                run('xcrun', ['stapler', 'validate', mountedAppPath]);
                run('spctl', ['-a', '-vv', '-t', 'execute', mountedAppPath]);
            }

            return identityMatches;
        });

        ok('DMG checksum', 'valid');
        ok('DMG installation layout', verification.layout.entries.join(', '));
        if (verification.callbackResult) {
            ok('mounted app identity', 'matches loose app bundle');
        }
        ok('DMG sha256', sha256(dmgPath));
    } catch (error) {
        fail(`DMG installation verification failed: ${error.message}`);
    }

    const stapler = run('xcrun', ['stapler', 'validate', dmgPath], { allowFailure: true });

    if (stapler.status === 0) {
        ok('notarization ticket', 'stapled');
    } else if (requireStapled) {
        fail(`notarization ticket is not stapled:\n${stapler.output}`);
    } else {
        warn('notarization ticket', 'not stapled yet');
    }

    if (requireStapled) {
        try {
            await verifyMacosDmgTrust(dmgPath);
            ok('Gatekeeper assessment', 'accepted');
        } catch (error) {
            fail(`DMG trust verification failed: ${error.message}`);
        }
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

        if (latest.version !== expectedVersion) {
            fail(`latest.json version ${latest.version || '(missing)'} does not match ${expectedVersion}`);
        }

        if (!latest.pub_date || Number.isNaN(Date.parse(latest.pub_date))) {
            fail('latest.json is missing a valid pub_date');
        }

        if (!platform?.url || !platform?.signature) {
            fail('latest.json is missing darwin-aarch64 url or signature');
        } else {
            let updaterUrl;

            try {
                updaterUrl = new URL(platform.url);
            } catch {
                fail(`latest.json updater URL is invalid: ${platform.url}`);
            }

            if (updaterUrl) {
                const expectedPath = `/aindaco1/social/releases/download/v${expectedVersion}/${encodeURIComponent(path.basename(updaterArtifactPath))}`;

                if (updaterUrl.protocol !== 'https:' || updaterUrl.hostname !== 'github.com' || updaterUrl.pathname !== expectedPath) {
                    fail(`latest.json updater URL does not match the v${expectedVersion} GitHub release asset: ${platform.url}`);
                }
            }

            if (hasSignature && platform.signature.trim() !== readFileSync(updaterSignaturePath, 'utf8').trim()) {
                fail('latest.json signature does not match the updater signature artifact');
            }

            ok('updater latest.json', `${latest.version} -> ${platform.url}`);
        }
    }

    if (hasArchive) {
        ok('updater archive sha256', sha256(updaterArtifactPath));

        const extractionRoot = mkdtempSync(path.join(os.tmpdir(), 'dust-wave-updater-verify-'));

        try {
            const extraction = run('/usr/bin/tar', ['-xzf', updaterArtifactPath, '-C', extractionRoot], {
                allowFailure: true,
            });

            if (extraction.status !== 0) {
                fail(`updater archive extraction failed:\n${extraction.output}`);
            } else {
                const appEntries = readdirSync(extractionRoot, { withFileTypes: true })
                    .filter((entry) => entry.isDirectory() && entry.name.endsWith('.app'));

                if (appEntries.length !== 1) {
                    fail(`updater archive must contain exactly one top-level .app bundle; found ${appEntries.length}`);
                } else {
                    const updaterAppPath = path.join(extractionRoot, appEntries[0].name);
                    run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', updaterAppPath]);
                    const identityMatches = compareAppIdentity('updater archive app', appIdentity(updaterAppPath));

                    if (identityMatches) {
                        ok('updater archive identity', 'matches loose app bundle');
                    }
                }
            }
        } finally {
            rmSync(extractionRoot, { recursive: true, force: true });
        }
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
