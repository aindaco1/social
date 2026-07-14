#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const ffmpegVersion = '8.1.2';
const sourceArchive = `ffmpeg-${ffmpegVersion}.tar.xz`;
const sourceUrl = `https://ffmpeg.org/releases/${sourceArchive}`;
const sourceSha256 = '464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c';
const force = args.includes('--force');
const supportedTarget = 'aarch64-apple-darwin';

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : null;
}

function run(command, commandArgs, options = {}) {
    const result = spawnSync(command, commandArgs, {
        cwd: options.cwd || projectRoot,
        env: options.env || process.env,
        encoding: 'utf8',
        shell: false,
        stdio: options.stdio || 'inherit',
    });

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }

    return result;
}

function commandOutput(command, commandArgs) {
    const result = spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });

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

    if (process.platform === 'darwin' && process.arch === 'arm64') {
        return supportedTarget;
    }

    return '';
}

async function fileSha256(filePath) {
    return createHash('sha256')
        .update(await readFile(filePath))
        .digest('hex');
}

async function ensureSourceArchive(cacheDirectory, archivePath) {
    await mkdir(cacheDirectory, { recursive: true });

    if (!existsSync(archivePath)) {
        run('curl', [
            '--fail',
            '--location',
            '--show-error',
            '--output',
            archivePath,
            sourceUrl,
        ]);
    }

    const actualSha = await fileSha256(archivePath);

    if (actualSha !== sourceSha256) {
        throw new Error(
            `FFmpeg source SHA-256 mismatch for ${archivePath}. Expected ${sourceSha256}, got ${actualSha}.`
        );
    }
}

async function main() {
    if (process.platform !== 'darwin') {
        throw new Error('Dust Wave currently builds bundled FFmpeg sidecars only on macOS.');
    }

    const targetTriple = currentTargetTriple();

    if (targetTriple !== supportedTarget) {
        throw new Error(`Dust Wave MVP sidecars support ${supportedTarget}; got ${targetTriple || 'unknown target'}.`);
    }

    const cacheDirectory = path.resolve(
        argValue('--cache-dir')
            || process.env.DUSTWAVE_FFMPEG_SOURCE_CACHE
            || path.join(os.tmpdir(), `dustwave-ffmpeg-lgpl-${ffmpegVersion}`),
    );
    const archivePath = path.join(cacheDirectory, sourceArchive);
    const sourceDirectory = path.join(cacheDirectory, `ffmpeg-${ffmpegVersion}`);
    const configurePrefix = `/tmp/dustwave-ffmpeg-lgpl-${ffmpegVersion}/install`;
    const ffmpegBinary = path.join(sourceDirectory, 'ffmpeg');
    const ffprobeBinary = path.join(sourceDirectory, 'ffprobe');

    if (force) {
        await rm(sourceDirectory, { recursive: true, force: true });
        await rm(configurePrefix, { recursive: true, force: true });
    }

    await ensureSourceArchive(cacheDirectory, archivePath);

    if (!existsSync(sourceDirectory)) {
        run('tar', ['-xf', archivePath, '-C', cacheDirectory]);
    }

    if (!existsSync(ffmpegBinary) || !existsSync(ffprobeBinary)) {
        const configureArgs = [
            `--prefix=${configurePrefix}`,
            '--cc=clang',
            '--disable-shared',
            '--enable-static',
            '--disable-doc',
            '--disable-debug',
            '--disable-ffplay',
            '--disable-network',
            '--disable-autodetect',
            '--disable-gpl',
            '--disable-nonfree',
            '--disable-iconv',
            '--disable-audiotoolbox',
            '--disable-videotoolbox',
            '--disable-avfoundation',
            '--enable-small',
            "--extra-cflags=-mmacosx-version-min=11.0",
            "--extra-ldflags=-mmacosx-version-min=11.0",
        ];

        run('./configure', configureArgs, { cwd: sourceDirectory });
        run('make', [`-j${Math.max(1, os.cpus().length)}`], { cwd: sourceDirectory });
    } else {
        console.log(`Using cached LGPL-only FFmpeg ${ffmpegVersion} build from ${sourceDirectory}`);
    }

    run(process.execPath, [
        path.join(scriptDirectory, 'prepare-media-sidecars.mjs'),
        '--target',
        targetTriple,
    ], {
        env: {
            ...process.env,
            DUSTWAVE_FFMPEG_BINARY: ffmpegBinary,
            DUSTWAVE_FFPROBE_BINARY: ffprobeBinary,
        },
    });

    console.log(`Built and staged LGPL-only FFmpeg ${ffmpegVersion} sidecars for ${targetTriple}`);
    console.log(`- ffmpeg sha256: ${await fileSha256(ffmpegBinary)}`);
    console.log(`- ffprobe sha256: ${await fileSha256(ffprobeBinary)}`);
}

main().catch((error) => {
    console.error(error.message || error);
    process.exit(1);
});
