import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import {
    MACOS_APP_IDENTIFIER,
    codesignArguments,
    developerIdIdentityFromOutput,
} from '../scripts/lib/macos-local-signing.mjs';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const packageJson = JSON.parse(await readFile(path.join(projectRoot, 'package.json'), 'utf8'));
const devRunner = await readFile(path.join(projectRoot, 'scripts', 'run-signed-macos-dev.mjs'), 'utf8');
const secretsSource = await readFile(path.join(projectRoot, 'src-tauri', 'src', 'secrets.rs'), 'utf8');

test('local signing resolves the installed Developer ID identity', () => {
    const output = '  1) ABCDEF "Developer ID Application: Dust Wave LLC (ABCDEFGHIJ)"';

    assert.equal(
        developerIdIdentityFromOutput(output),
        'Developer ID Application: Dust Wave LLC (ABCDEFGHIJ)',
    );
});

test('stable local signatures use the production identifier', () => {
    const args = codesignArguments('/tmp/Dust Wave Social.app', {
        deep: true,
        identity: 'Developer ID Application: Dust Wave LLC (ABCDEFGHIJ)',
    });

    assert.deepEqual(args, [
        '--force',
        '--deep',
        '--sign',
        'Developer ID Application: Dust Wave LLC (ABCDEFGHIJ)',
        '--identifier',
        MACOS_APP_IDENTIFIER,
        '/tmp/Dust Wave Social.app',
    ]);
});

test('desktop development signs before enabling Keychain access', () => {
    assert.match(packageJson.scripts['desktop:dev'], /CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER/);
    assert.ok(packageJson.scripts['desktop:dev'].includes('run-signed-macos-dev.mjs'));
    assert.ok(devRunner.includes("childEnv.DUSTWAVE_KEYCHAIN_MODE = 'keychain'"));
    assert.ok(devRunner.includes("childEnv.DUSTWAVE_KEYCHAIN_MODE = 'environment'"));
    assert.ok(devRunner.includes('refusing to launch an unstable Keychain requester'));
    assert.ok(secretsSource.includes('unsigned_debug_builds_default_to_environment_only_credentials'));
});
