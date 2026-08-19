import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const appSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'App.vue'),
    'utf8',
);
const updateStatusButtonSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'UpdateStatusButton.vue'),
    'utf8',
);

function sourceBetween(start, end) {
    const startIndex = appSource.indexOf(start);
    const endIndex = appSource.indexOf(end, startIndex + start.length);

    assert.notEqual(startIndex, -1, `Missing source marker: ${start}`);
    assert.notEqual(endIndex, -1, `Missing source marker: ${end}`);

    return appSource.slice(startIndex, endIndex);
}

test('TikTok service setup defaults to the deployed Dust Wave broker', () => {
    const match = appSource.match(/const dustWaveTikTokBrokerUrl = '([^']+)'/);

    assert.ok(match, 'Dust Wave TikTok broker URL constant is missing');
    assert.equal(match[1], 'https://dustwave-tiktok-broker.jogo.workers.dev');
    assert.ok(appSource.includes("value: `${dustWaveTikTokBrokerUrl}/api/tiktok/oauth/callback`"));
    assert.ok(appSource.includes('defaultValue: dustWaveTikTokBrokerUrl'));
    assert.ok(appSource.includes('placeholder: dustWaveTikTokBrokerUrl'));
});

test('TikTok desktop credentials exclude the client secret', () => {
    const tiktokService = sourceBetween("id: 'tiktok'", "id: 'unsplash'");

    assert.ok(tiktokService.includes("field: 'client_id'"));
    assert.equal(tiktokService.includes("field: 'client_secret'"), false);
    assert.ok(tiktokService.includes('Store TikTok client secret only in the Cloudflare broker'));
});

test('TikTok readiness and onboarding require broker-backed analytics inputs', () => {
    assert.ok(appSource.includes("return Boolean(status?.configured && serviceActiveValue(serviceName) && tiktokBrokerBaseUrl())"));
    assert.ok(appSource.includes("['tiktok', '', '', '', 'assisted', 'yes', 'Requires broker-issued connection credential for analytics.']"));
    assert.ok(appSource.includes('Broker connection credential is required'));
    assert.ok(appSource.includes('/api/tiktok/oauth/start'));
});

test('Instagram is exposed as a first-class Meta account type', () => {
    assert.ok(appSource.includes("instagram: ["));
    assert.ok(appSource.includes("connectInstagramAccounts"));
    assert.ok(appSource.includes("refresh_instagram_account"));
    assert.ok(appSource.includes("import_instagram_account_data"));
    assert.ok(appSource.includes("['instagram', '', '', '', 'yes', 'yes', 'Use Facebook OAuth, then choose connected Instagram accounts.']"));
});

test('media staging service requires an HTTPS Worker base URL', () => {
    const mediaStagingService = sourceBetween("id: 'media_staging'", "id: 'twitter'");

    assert.ok(mediaStagingService.includes("label: 'Media Staging'"));
    assert.ok(mediaStagingService.includes("field: 'client_secret'"));
    assert.ok(mediaStagingService.includes("field: 'base_url'"));
    assert.ok(appSource.includes("normalizedServiceBaseUrl('media_staging', 'base_url')"));
    assert.ok(appSource.includes("serviceName === 'media_staging'"));
});

test('local AI media labs are opt-in and use bundled LiteRT assets', () => {
    assert.ok(appSource.includes('local_ai_media_labs'));
    assert.ok(appSource.includes("await import('@litertjs/core')"));
    assert.ok(appSource.includes("await litert.loadLiteRt('./litert/wasm/')"));
    assert.ok(appSource.includes("fetch('./litert/models/manifest.json'"));
    assert.ok(appSource.includes("loadAndCompile(`./litert/models/${bundle.modelFile.path}`"));
    assert.ok(appSource.includes('new Tensor(inputBytes, inputShape)'));
    assert.ok(appSource.includes("toDataURL('image/png')"));
    assert.ok(appSource.includes('compiled_model'));
    assert.ok(appSource.includes("invoke('save_local_ai_model_upscale_derivative'"));
    assert.ok(appSource.includes('cancelLocalAiOperation'));
    assert.ok(appSource.includes("invoke('local_ai_media_search'"));
    assert.ok(appSource.includes('Semantic media search'));
});

test('topbar updater uses the Podcast Visualizer icon and compact check-or-install flow', () => {
    const updateAction = sourceBetween('const checkOrInstallSoftwareUpdate', 'const exportSystemLog');

    assert.ok(appSource.includes("import UpdateStatusButton from '@desktop/components/UpdateStatusButton.vue'"));
    assert.ok(appSource.includes('@activate="checkOrInstallSoftwareUpdate"'));
    assert.ok(updateAction.includes('softwareUpdateAvailable.value'));
    assert.ok(updateAction.includes('await installSoftwareUpdate()'));
    assert.ok(updateAction.includes('await checkSoftwareUpdate()'));
    assert.ok(appSource.includes('{ timeout: 300000 }'));
    assert.ok(updateStatusButtonSource.includes('data-icon="arrow-down-circle"'));
    assert.ok(updateStatusButtonSource.includes("return props.available ? 'Install' : 'Update'"));
    assert.ok(updateStatusButtonSource.includes(':aria-label="actionDescription"'));
    assert.ok(updateStatusButtonSource.includes(':aria-busy="busy"'));
});
