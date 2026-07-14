#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { constants, existsSync } from 'node:fs';
import { access, chmod, copyFile, mkdir, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const binariesDirectory = path.join(projectRoot, 'src-tauri', 'binaries');
const args = process.argv.slice(2);
const checkOnly = args.includes('--check');
const allowDynamicSystemBinaries = args.includes('--allow-dynamic-system-binaries');
const disallowedLgplOnlyFlags = [
    '--enable-gpl',
    '--enable-nonfree',
    '--enable-libx264',
    '--enable-libx265',
    '--enable-libxvid',
    '--enable-libvidstab',
    '--enable-libxavs',
    '--enable-libxavs2',
];

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : null;
}

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        encoding: 'utf8',
        stdio: options.stdio || 'pipe',
        shell: false,
    });
}

function commandOutput(command, commandArgs) {
    const result = run(command, commandArgs);

    if (result.status !== 0) {
        return '';
    }

    return String(result.stdout || '').trim();
}

function currentTargetTriple() {
    const explicit = argValue('--target') || process.env.TAURI_TARGET_TRIPLE;

    if (explicit) {
        return explicit;
    }

    const hostTuple = commandOutput('rustc', ['--print', 'host-tuple']);

    if (hostTuple) {
        return hostTuple;
    }

    const rustcVersion = commandOutput('rustc', ['-Vv']);
    const hostLine = rustcVersion
        .split('\n')
        .find((line) => line.startsWith('host: '));

    if (hostLine) {
        return hostLine.replace('host: ', '').trim();
    }

    if (process.platform === 'darwin' && process.arch === 'arm64') {
        return 'aarch64-apple-darwin';
    }

    if (process.platform === 'darwin' && process.arch === 'x64') {
        return 'x86_64-apple-darwin';
    }

    if (process.platform === 'linux' && process.arch === 'x64') {
        return 'x86_64-unknown-linux-gnu';
    }

    if (process.platform === 'linux' && process.arch === 'arm64') {
        return 'aarch64-unknown-linux-gnu';
    }

    if (process.platform === 'win32' && process.arch === 'x64') {
        return 'x86_64-pc-windows-msvc';
    }

    if (process.platform === 'win32' && process.arch === 'arm64') {
        return 'aarch64-pc-windows-msvc';
    }

    throw new Error('Unable to determine the Rust target triple. Pass --target <triple>.');
}

function sidecarFilename(binaryName, targetTriple) {
    return `${binaryName}-${targetTriple}${targetTriple.includes('windows') ? '.exe' : ''}`;
}

async function assertExecutable(binaryPath, label) {
    const result = run(binaryPath, ['-version']);

    if (result.status !== 0) {
        const detail = String(result.stderr || result.stdout || '').trim();
        throw new Error(`${label} did not run successfully at ${binaryPath}${detail ? `: ${detail}` : ''}`);
    }

    return String(result.stdout || result.stderr || '').trim();
}

function licenseOutput(binaryPath) {
    const result = run(binaryPath, ['-L']);

    if (result.status !== 0) {
        return '';
    }

    return String(result.stdout || result.stderr || '').trim();
}

function configurationLine(versionOutput) {
    return versionOutput
        .split('\n')
        .find((line) => line.trim().startsWith('configuration:')) || '';
}

function versionLine(versionOutput) {
    return versionOutput.split('\n').find(Boolean) || '';
}

function assertLgplOnly(binaryPath, label, versionOutput) {
    const config = configurationLine(versionOutput);
    const lowerConfig = config.toLowerCase();
    const matchedFlag = disallowedLgplOnlyFlags.find((flag) => lowerConfig.includes(flag));

    if (matchedFlag) {
        throw new Error(`${label} at ${binaryPath} is not LGPL-only; configuration includes ${matchedFlag}.`);
    }

    const license = licenseOutput(binaryPath);

    if (!license) {
        throw new Error(`${label} at ${binaryPath} did not expose FFmpeg license text with -L.`);
    }

    const normalizedLicense = license.replace(/\s+/g, ' ');

    if (/under the terms of the GNU General Public License\b/i.test(normalizedLicense)) {
        throw new Error(`${label} at ${binaryPath} reports GPL licensing. Use an LGPL-only build.`);
    }

    if (!/GNU Lesser General Public License/i.test(normalizedLicense)) {
        throw new Error(`${label} at ${binaryPath} did not report LGPL licensing. Use an LGPL-only FFmpeg build.`);
    }

    return license;
}

function optionalPackageBinary(packageName, exportMapper = (value) => value) {
    try {
        return exportMapper(require(packageName));
    } catch {
        return '';
    }
}

function pathCommand(binaryName) {
    const command = process.platform === 'win32' ? 'where' : 'which';
    const result = commandOutput(command, [binaryName]);

    return result.split(/\r?\n/).find(Boolean) || '';
}

function resolveSourceBinary(binaryName) {
    const explicit = binaryName === 'ffmpeg'
        ? argValue('--ffmpeg') || process.env.DUSTWAVE_FFMPEG_BINARY || process.env.FFMPEG_PATH
        : argValue('--ffprobe') || process.env.DUSTWAVE_FFPROBE_BINARY || process.env.FFPROBE_PATH;

    if (explicit) {
        return path.resolve(explicit);
    }

    const packageBinary = binaryName === 'ffmpeg'
        ? optionalPackageBinary('ffmpeg-static')
        : optionalPackageBinary('ffprobe-static', (value) => value.path);

    if (packageBinary) {
        return path.resolve(packageBinary);
    }

    const onPath = pathCommand(binaryName);

    return onPath ? path.resolve(onPath) : '';
}

async function assertReadableFile(binaryPath, label) {
    if (!binaryPath) {
        throw new Error(`${label} source binary was not found.`);
    }

    await access(binaryPath, constants.R_OK);
    const stats = await stat(binaryPath);

    if (!stats.isFile()) {
        throw new Error(`${label} source is not a file: ${binaryPath}`);
    }
}

function macDynamicDependencies(binaryPath) {
    if (process.platform !== 'darwin' || !existsSync('/usr/bin/otool')) {
        return [];
    }

    const result = run('/usr/bin/otool', ['-L', binaryPath]);

    if (result.status !== 0) {
        return [];
    }

    return String(result.stdout || '')
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line.includes('/opt/homebrew/') || line.includes('/usr/local/Cellar/'));
}

function assertPortableEnough(binaryPath, label) {
    const dynamicDependencies = macDynamicDependencies(binaryPath);

    if (!dynamicDependencies.length || allowDynamicSystemBinaries) {
        return;
    }

    throw new Error(
        `${label} at ${binaryPath} is dynamically linked to Homebrew libraries. ` +
        'Use a portable/static build, or rerun with --allow-dynamic-system-binaries for local-only testing.'
    );
}

async function checkStagedSidecar(binaryName, targetTriple) {
    const destination = path.join(binariesDirectory, sidecarFilename(binaryName, targetTriple));

    await assertReadableFile(destination, binaryName);
    const version = await assertExecutable(destination, binaryName);
    assertLgplOnly(destination, binaryName, version);
    assertPortableEnough(destination, binaryName);

    return destination;
}

async function stageSidecar(binaryName, targetTriple) {
    const source = resolveSourceBinary(binaryName);
    await assertReadableFile(source, binaryName);
    const sourceVersion = await assertExecutable(source, binaryName);
    const sourceLicense = assertLgplOnly(source, binaryName, sourceVersion);
    assertPortableEnough(source, binaryName);

    await mkdir(binariesDirectory, { recursive: true });

    const destination = path.join(binariesDirectory, sidecarFilename(binaryName, targetTriple));
    await copyFile(source, destination);
    await chmod(destination, 0o755);
    const stagedVersion = await assertExecutable(destination, binaryName);
    assertLgplOnly(destination, binaryName, stagedVersion);

    return {
        binaryName,
        source,
        destination,
        version: versionLine(stagedVersion),
        configuration: configurationLine(stagedVersion),
        license: sourceLicense.split('\n').slice(0, 5).join('\n'),
    };
}

async function main() {
    const targetTriple = currentTargetTriple();

    if (checkOnly) {
        const ffmpeg = await checkStagedSidecar('ffmpeg', targetTriple);
        const ffprobe = await checkStagedSidecar('ffprobe', targetTriple);
        console.log(`Media sidecars ready for ${targetTriple}`);
        console.log(`- ${ffmpeg}`);
        console.log(`- ${ffprobe}`);
        return;
    }

    const staged = [
        await stageSidecar('ffmpeg', targetTriple),
        await stageSidecar('ffprobe', targetTriple),
    ];

    await writeFile(
        path.join(binariesDirectory, 'SIDECARS.local.json'),
        `${JSON.stringify({ targetTriple, stagedAt: new Date().toISOString(), staged }, null, 2)}\n`
    );

    console.log(`Prepared media sidecars for ${targetTriple}`);
    for (const item of staged) {
        console.log(`- ${item.binaryName}: ${item.destination}`);
    }
}

main().catch((error) => {
    console.error(error.message || error);
    process.exit(1);
});
