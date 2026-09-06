#!/usr/bin/env node
// Build-time only. End users never download runtimes or install Python/Xcode.
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile, copyFile, mkdtemp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir, homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
export const nativeLiteRt = {
    version: '2.1.6', minimumMacOS: '14.0',
    wheel: { url: 'https://files.pythonhosted.org/packages/52/d5/164aaf69f60f72b7076900ef1cc6153bf50d82cd15202bdf1239c0dbfb1c/ai_edge_litert-2.1.6-cp312-cp312-macosx_12_0_arm64.whl', sha256: '5adf0c9afde6151dc7f2989d039c800f3060d98d40bb5dfc95e426ad4eb3680b' },
    sdk: { url: 'https://github.com/google-ai-edge/LiteRT/releases/download/v2.1.6/litert_cc_sdk.zip', sha256: '2cbde8fc18cd3d6ffbab6bcdecb92b1d49b198e50a7bdf46e01cd329c657aca8' },
    librarySha256: '1e2bae791469e9891fb30b8c36b6a2e202dd55bc49acdf5cf6be1b9dce6b08e2',
};
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function run(command, args) {
    const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout: 90000 });
    if (result.status !== 0) throw new Error(result.stderr || result.error?.message || `${command} failed`);
    return result.stdout;
}
async function prepare() {
    if (process.platform !== 'darwin' || process.arch !== 'arm64') {
        console.log('Native LiteRT packaging targets Apple Silicon; WebAssembly remains available.');
        return;
    }
    const destination = path.join(root, 'src-tauri/native-runtime');
    const executable = path.join(root, 'src-tauri/binaries/social-litert-aarch64-apple-darwin');
    const library = path.join(destination, 'libLiteRt.dylib');
    const receipt = path.join(destination, 'build.json');
    const sourceSha = hash(Buffer.concat(await Promise.all([
        'native/local_ai_runner.cpp', 'native/include/litert/build_common/build_config.h', 'scripts/prepare-native-litert.mjs',
    ].map(name => readFile(path.join(root, name))))));
    if (existsSync(receipt) && existsSync(executable) && existsSync(library)) {
        const record = JSON.parse(await readFile(receipt, 'utf8'));
        if (record.sourceSha === sourceSha && record.executableSha === hash(await readFile(executable)) && hash(await readFile(library)) === nativeLiteRt.librarySha256) {
            console.log('Native LiteRT helper and runtime verified.');
            return;
        }
    }
    if (process.argv.includes('--check')) throw new Error('Native LiteRT assets missing or stale; run npm run local-ai:native:prepare');
    const temporary = await mkdtemp(path.join(tmpdir(), 'social-litert-build-'));
    try {
        const cache = path.join(homedir(), 'Library/Caches/DustWaveSocial/litert');
        await mkdir(cache, { recursive: true });
        for (const [name, source] of Object.entries({ wheel: nativeLiteRt.wheel, sdk: nativeLiteRt.sdk })) {
            const cached = path.join(cache, source.sha256 + '.zip');
            let bytes = existsSync(cached) ? await readFile(cached) : null;
            if (!bytes || hash(bytes) !== source.sha256) {
                const response = await fetch(source.url, { signal: AbortSignal.timeout(60000) });
                if (!response.ok) throw new Error(`LiteRT ${name} download failed (${response.status})`);
                bytes = Buffer.from(await response.arrayBuffer());
            }
            if (hash(bytes) !== source.sha256) throw new Error(`LiteRT ${name} checksum mismatch`);
            await writeFile(cached, bytes);
            await writeFile(path.join(temporary, name + '.zip'), bytes);
        }
        run('unzip', ['-q', path.join(temporary, 'wheel.zip'), 'ai_edge_litert/libLiteRt.dylib', '-d', temporary]);
        run('unzip', ['-q', path.join(temporary, 'sdk.zip'), '-d', temporary]);
        const bytes = await readFile(path.join(temporary, 'ai_edge_litert/libLiteRt.dylib'));
        if (hash(bytes) !== nativeLiteRt.librarySha256) throw new Error('LiteRT library checksum mismatch');
        await mkdir(destination, { recursive: true });
        await mkdir(path.dirname(executable), { recursive: true });
        await copyFile(path.join(temporary, 'ai_edge_litert/libLiteRt.dylib'), library);
        run('xcrun', ['clang++', '-std=c++17', '-O2', '-mmacosx-version-min=14.0', '-I', 'native/include', '-I', path.join(temporary, 'litert_cc_sdk'),
            'native/local_ai_runner.cpp', '-L', destination, '-lLiteRt', '-Wl,-rpath,@executable_path/../Frameworks', '-Wl,-rpath,@executable_path/../native-runtime', '-o', executable]);
        await writeFile(receipt, JSON.stringify({ ...nativeLiteRt, sourceSha, executableSha: hash(await readFile(executable)) }, null, 2) + '\n');
        console.log('Built pinned LiteRT 2.1.6 CPU helper for macOS 14+; no Python runtime bundled.');
    } finally { await rm(temporary, { recursive: true, force: true }); }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await prepare();
