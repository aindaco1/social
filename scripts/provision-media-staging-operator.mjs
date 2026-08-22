#!/usr/bin/env node

import { randomBytes } from 'node:crypto';
import { gitRemoteRepoSlug } from './release-repo.js';
import {
    cloudflareSecretNames,
    mediaStagingConfigPath,
    projectRoot,
    requireSuccess,
    run,
    storeLocalMediaStagingToken,
} from './lib/media-staging-operations.mjs';

const args = new Set(process.argv.slice(2));
const apply = args.has('--apply');
const replaceAdditive = args.has('--replace-additive');
const secretName = 'MEDIA_STAGING_TOKEN_NEXT';
const repo = process.env.DUSTWAVE_RELEASE_REPO || gitRemoteRepoSlug(projectRoot) || '';

if (!repo) {
    throw new Error('Unable to determine the GitHub owner/repository.');
}

requireSuccess(run('npm', ['run', 'media:staging:config:check']), 'Preparing Media Staging configuration');
const existingSecrets = cloudflareSecretNames();

console.log(`${apply ? 'Provisioning' : 'Plan for'} additive Media Staging operator access`);
console.log(`Worker secret: ${secretName}`);
console.log(`GitHub secret: ${repo}/${secretName}`);
console.log('Local destination: macOS Keychain for signed Dust Wave Social builds');

if (existingSecrets.has(secretName) && !replaceAdditive) {
    throw new Error(`${secretName} already exists. Refusing to replace a credential that may be used by another installation.`);
}

if (replaceAdditive) {
    console.warn('Replacing the additive credential by explicit request. The primary credential remains unchanged.');
}

if (!apply) {
    console.log('No changes made. Re-run with --apply to deploy pairing support and provision the additive credential.');
    process.exit(0);
}

requireSuccess(run('npx', [
    'wrangler',
    'deploy',
    '--config',
    mediaStagingConfigPath,
    '--keep-vars',
]), 'Deploying Media Staging pairing support');

const token = `dwmo_${randomBytes(32).toString('base64url')}`;

storeLocalMediaStagingToken(token);

requireSuccess(run('gh', [
    'secret',
    'set',
    secretName,
    '--repo',
    repo,
], { input: token }), 'Saving the additive operator credential in GitHub');

requireSuccess(run('npx', [
    'wrangler',
    'secret',
    'put',
    secretName,
    '--config',
    mediaStagingConfigPath,
], { input: token }), 'Creating the additive Worker secret');

if (!cloudflareSecretNames().has(secretName)) {
    throw new Error('Wrangler completed, but the additive Worker secret is not listed.');
}

console.log('Provisioning complete. The primary credential remains valid, and this Mac now uses the additive credential.');
