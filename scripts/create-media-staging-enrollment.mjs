#!/usr/bin/env node

import { hostname } from 'node:os';
import {
    defaultMediaStagingUrl,
    readLocalMediaStagingToken,
    requireSuccess,
    run,
} from './lib/media-staging-operations.mjs';

const args = process.argv.slice(2);
const print = args.includes('--print');
const labelIndex = args.indexOf('--label');
const urlIndex = args.indexOf('--url');
const label = labelIndex >= 0 ? String(args[labelIndex + 1] || '').trim() : hostname();
const baseUrl = (urlIndex >= 0 ? String(args[urlIndex + 1] || '') : defaultMediaStagingUrl)
    .trim()
    .replace(/\/+$/, '');
const operatorToken = String(process.env.DUSTWAVE_MEDIA_STAGING_TOKEN || '').trim()
    || readLocalMediaStagingToken();

if (!baseUrl.startsWith('https://')) {
    throw new Error('The Media Staging URL must use HTTPS.');
}

const response = await fetch(`${baseUrl}/api/enrollments`, {
    method: 'POST',
    headers: {
        authorization: `Bearer ${operatorToken}`,
        'content-type': 'application/json',
    },
    body: JSON.stringify({ label, ttl_seconds: 900 }),
});
const payload = await response.json().catch(() => ({}));

if (!response.ok || !payload.enrollment_code) {
    throw new Error(`Creating a one-time setup code failed: ${payload.error || `HTTP ${response.status}`}`);
}

if (print) {
    console.log(payload.enrollment_code);
} else {
    requireSuccess(run('/usr/bin/pbcopy', [], { input: payload.enrollment_code }), 'Copying the one-time setup code');
    console.log('One-time setup code copied to the clipboard.');
}

console.log(`Expires: ${payload.expires_at}`);
console.log('Send it through a private channel; it can pair one Mac and cannot be reused.');
