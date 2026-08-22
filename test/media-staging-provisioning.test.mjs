import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (relativePath) => readFileSync(path.join(root, relativePath), 'utf8');
const provisionSource = read('scripts/provision-media-staging-operator.mjs');
const operationsSource = read('scripts/lib/media-staging-operations.mjs');
const enrollmentSource = read('scripts/create-media-staging-enrollment.mjs');
const workflowSource = read('.github/workflows/media-staging.yml');

test('operator provisioning is additive and refuses an implicit rotation', () => {
    assert.ok(provisionSource.includes("const secretName = 'MEDIA_STAGING_TOKEN_NEXT'"));
    assert.ok(provisionSource.includes("const replaceAdditive = args.has('--replace-additive')"));
    assert.ok(provisionSource.includes('Refusing to replace a credential that may be used by another installation.'));
    assert.equal(provisionSource.includes("secretName = 'MEDIA_STAGING_TOKEN'"), false);
    assert.ok(provisionSource.includes("randomBytes(32).toString('base64url')"));
});

test('operator provisioning never passes or prints the credential as a command argument', () => {
    assert.ok(provisionSource.includes("{ input: token }"));
    assert.equal(provisionSource.includes('console.log(token)'), false);
    assert.equal(provisionSource.includes("'-w', token"), false);
    assert.ok(operationsSource.includes("'swiftc'"));
    assert.ok(operationsSource.includes('signLocalMacosCode(helperPath)'));
    assert.ok(operationsSource.includes("{ input: `${token}\\n` }"));
    assert.equal(operationsSource.includes("'add-generic-password'"), false);
    assert.equal(operationsSource.includes("'find-generic-password'"), false);
    assert.ok(operationsSource.includes("'get',\n            keychainService"));
});

test('deployment preserves the primary secret and CI carries the additive secret forward', () => {
    const deployIndex = provisionSource.indexOf("'deploy'");
    const secretPutIndex = provisionSource.indexOf("'put'", deployIndex + 1);

    assert.notEqual(deployIndex, -1);
    assert.ok(secretPutIndex > deployIndex);
    assert.ok(provisionSource.includes("'--keep-vars'"));
    assert.ok(workflowSource.includes('MEDIA_STAGING_TOKEN_NEXT: ${{ secrets.MEDIA_STAGING_TOKEN_NEXT }}'));
    assert.ok(workflowSource.includes("printf 'MEDIA_STAGING_TOKEN_NEXT=%s\\n'"));
});

test('one-time enrollment uses the paired Keychain credential and avoids terminal output by default', () => {
    assert.ok(enrollmentSource.includes('readLocalMediaStagingToken()'));
    assert.ok(enrollmentSource.includes("fetch(`${baseUrl}/api/enrollments`"));
    assert.ok(enrollmentSource.includes("run('/usr/bin/pbcopy'"));
    assert.ok(enrollmentSource.includes("const print = args.includes('--print')"));
    assert.ok(enrollmentSource.includes('can pair one Mac and cannot be reused'));
});
