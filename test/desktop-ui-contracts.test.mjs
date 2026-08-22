import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { Update } from '@tauri-apps/plugin-updater';
import { isProxy, shallowRef } from 'vue';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const appSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'App.vue'),
    'utf8',
);
const updateStatusButtonSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'UpdateStatusButton.vue'),
    'utf8',
);
const confirmDialogSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'ConfirmDialog.vue'),
    'utf8',
);
const contextualEditorSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'ContextualEditor.vue'),
    'utf8',
);
const postDetailModalSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'PostDetailModal.vue'),
    'utf8',
);
const workspaceTabsSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'WorkspaceTabs.vue'),
    'utf8',
);
const desktopStylesSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'styles.css'),
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

test('add account modal chooses one provider before showing its readable form', () => {
    assert.ok(appSource.includes('const accountProviderChoices = ['));
    assert.ok(appSource.includes('v-if="!activeAccountProvider" class="account-provider-choice-list"'));
    assert.ok(appSource.includes("v-if=\"activeAccountProvider === 'twitter'\""));
    assert.ok(appSource.includes("v-if=\"activeAccountProvider === 'facebook'\""));
    assert.ok(appSource.includes("v-if=\"activeAccountProvider === 'mastodon'\""));
    assert.ok(appSource.includes("v-if=\"activeAccountProvider === 'tiktok'\""));
    assert.match(
        desktopStylesSource,
        /\.account-provider-choice-list\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/s,
    );
    assert.match(
        desktopStylesSource,
        /\.account-provider-grid\s*{[^}]*grid-template-columns:\s*minmax\(0, 1fr\)/s,
    );
    assert.match(
        desktopStylesSource,
        /\.twitter-oauth-form\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/s,
    );
    assert.match(
        desktopStylesSource,
        /\.facebook-oauth-form\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/s,
    );
    assert.match(
        desktopStylesSource,
        /\.mastodon-app-form\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/s,
    );
    assert.match(
        desktopStylesSource,
        /\.tiktok-connection-form\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/s,
    );
    assert.ok(desktopStylesSource.includes('.tiktok-connection-form input,'));
    assert.match(
        desktopStylesSource,
        /@media \(max-width: 900px\)[\s\S]*\.account-provider-choice-list,[\s\S]*\.tiktok-connection-form\s*{\s*grid-template-columns:\s*1fr;/,
    );
});

test('shared workspace tabs and confirmation dialog preserve accessible interaction semantics', () => {
    assert.ok(appSource.includes("import WorkspaceTabs from '@desktop/components/WorkspaceTabs.vue'"));
    assert.ok(appSource.includes("import ConfirmDialog from '@desktop/components/ConfirmDialog.vue'"));
    assert.ok(workspaceTabsSource.includes('role="tablist"'));
    assert.ok(workspaceTabsSource.includes('role="tab"'));
    assert.ok(workspaceTabsSource.includes(':aria-selected="modelValue === tab.id"'));
    assert.ok(confirmDialogSource.includes('role="dialog"'));
    assert.ok(confirmDialogSource.includes('aria-modal="true"'));
    assert.ok(confirmDialogSource.includes('@keydown.esc.prevent="emit(\'cancel\')"'));
    assert.ok(appSource.includes('const requestConfirmation = ({'));
    assert.equal(appSource.includes('window.confirm'), false);
});

test('contextual edits reuse one accessible editor and preserve the surrounding workflow', () => {
    assert.ok(appSource.includes("import ContextualEditor from '@desktop/components/ContextualEditor.vue'"));
    assert.equal((appSource.match(/<ContextualEditor/g) || []).length, 2);
    assert.ok(contextualEditorSource.includes('@keydown.esc.stop.prevent="cancel"'));
    assert.ok(contextualEditorSource.includes('[data-contextual-autofocus]'));
    assert.ok(contextualEditorSource.includes('returnFocusTarget = document.activeElement'));
    assert.ok(contextualEditorSource.includes('returnFocusTarget.focus()'));
    assert.ok(contextualEditorSource.includes(':aria-busy="busy"'));
    assert.ok(appSource.includes('const openPostScheduleEditor = (post) =>'));
    assert.ok(appSource.includes('v-if="editingSchedulePostUuid === post.uuid"'));
    assert.ok(appSource.includes('const editSelectedPostFromDetail = async () =>'));
    assert.ok(appSource.includes('contextualEditOrigin.value = originView'));
    assert.ok(appSource.includes('Back to {{ contextualEditOriginLabel }}'));
    assert.ok(postDetailModalSource.includes('Edit in composer'));
    assert.ok(postDetailModalSource.includes("defineEmits(['close', 'edit'])"));
    assert.match(desktopStylesSource, /\.post-schedule-editor\s*{[^}]*grid-column:\s*1 \/ -1/s);
    assert.match(desktopStylesSource, /\.tag-edit-form \.contextual-editor-fields\s*{[^}]*grid-template-columns:/s);
});

test('Instagram is exposed as a first-class Meta account type', () => {
    assert.ok(appSource.includes("instagram: ["));
    assert.ok(appSource.includes("connectInstagramAccounts"));
    assert.ok(appSource.includes("refresh_instagram_account"));
    assert.ok(appSource.includes("import_instagram_account_data"));
    assert.ok(appSource.includes("['instagram', '', '', '', 'yes', 'yes', 'Use Facebook OAuth, then choose connected Instagram accounts.']"));
});

test('Instagram local media offers one-time pairing and retains advanced token setup', () => {
    const mediaStagingService = sourceBetween("id: 'media_staging'", "id: 'twitter'");

    assert.ok(mediaStagingService.includes("label: 'Instagram Local Media'"));
    assert.ok(mediaStagingService.includes('managed: true'));
    assert.ok(mediaStagingService.includes("field: 'client_secret'"));
    assert.ok(mediaStagingService.includes("field: 'base_url'"));
    assert.ok(appSource.includes("normalizedServiceBaseUrl('media_staging', 'base_url')"));
    assert.ok(appSource.includes("serviceName === 'media_staging'"));
    assert.ok(appSource.includes('const enrollMediaStaging = async () =>'));
    assert.ok(appSource.includes("invoke('enroll_media_staging'"));
    assert.ok(appSource.includes('autocomplete="one-time-code"'));
    assert.ok(appSource.includes('The setup code cannot be reused.'));
    assert.ok(appSource.includes('This Mac is paired'));
    assert.ok(appSource.includes('No Cloudflare account or Wrangler setup is required.'));
    assert.ok(appSource.includes("mediaStagingAdvancedOpen ? 'Hide Advanced Manual Setup' : 'Advanced Manual Setup'"));
    assert.ok(appSource.includes('activeServiceDefinition.managed ? mediaStagingAdvancedOpen : (!activeServiceIsReady || activeServiceSettingsOpen)'));
});

test('provider diagnostics stay contextual instead of repeating every service in a separate matrix', () => {
    assert.ok(appSource.includes('const activeServiceCredentialSummary = computed(() =>'));
    assert.ok(appSource.includes('class="service-credential-summary"'));
    assert.ok(appSource.includes('const activeServiceSettingsOpen = computed(() =>'));
    assert.ok(appSource.includes('Edit Settings'));
    assert.ok(appSource.includes('<summary>Share setup</summary>'));
    assert.equal(appSource.includes('credential-diagnostics'), false);
    assert.equal(appSource.includes('configuredCredentialCount'), false);
    assert.equal(desktopStylesSource.includes('.credential-card'), false);
    assert.equal(desktopStylesSource.includes('.credential-grid'), false);
});

test('background maintenance stays quiet when it finds nothing to clean up', () => {
    assert.ok(appSource.includes('desktopMaintenanceSummaryVisible.value = !background || changed'));
    assert.ok(appSource.includes('Maintenance complete. Nothing needed cleanup.'));
    assert.ok(appSource.includes('value: formatTimestamp(autoMaintenanceLastRun.value)'));
    assert.equal(appSource.includes('last check {{ autoMaintenanceLastRun }}'), false);
});

test('every service configuration uses one explicit save with complete feedback', () => {
    const serviceDefinitionsSource = sourceBetween('const serviceDefinitions = [', 'const serviceConfigurationDefaults');
    const serviceSaveFlow = sourceBetween('const saveServiceSettings', 'const openServiceUrl');
    const servicesMarkup = sourceBetween(
        "<article v-if=\"activeView === 'connections' && activeConnectionTab === 'services'\"",
        "<article v-if=\"activeView === 'posts'\"",
    );

    for (const serviceName of ['facebook', 'media_staging', 'twitter', 'tiktok', 'unsplash', 'klipy']) {
        assert.ok(serviceDefinitionsSource.includes(`id: '${serviceName}'`));
    }

    assert.ok(servicesMarkup.includes('@submit.prevent="saveServiceSettings(activeServiceDefinition.id)"'));
    assert.ok(servicesMarkup.includes('v-model="serviceActiveDrafts[activeServiceDefinition.id]"'));
    assert.ok(servicesMarkup.includes('Save ${activeServiceDefinition.label} Settings'));
    assert.ok(servicesMarkup.includes('role="status"'));
    assert.ok(servicesMarkup.includes('role="alert"'));
    assert.equal(servicesMarkup.includes('saveServiceCredential'), false);
    assert.equal(servicesMarkup.includes('Save Service'), false);

    const credentialSaveIndex = serviceSaveFlow.indexOf("invoke('save_service_credential'");
    const configurationSaveIndex = serviceSaveFlow.indexOf("invoke('save_service'");

    assert.notEqual(credentialSaveIndex, -1);
    assert.ok(configurationSaveIndex > credentialSaveIndex);
    assert.ok(serviceSaveFlow.includes('missingCredentials'));
    assert.ok(serviceSaveFlow.includes('active: previousActive'));
    assert.ok(serviceSaveFlow.includes('remain in Keychain'));
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
    const installAction = sourceBetween('const installSoftwareUpdate', 'const checkOrInstallSoftwareUpdate');

    assert.ok(appSource.includes("import UpdateStatusButton from '@desktop/components/UpdateStatusButton.vue'"));
    assert.match(appSource, /import\s*\{[^}]*\bChannel\b[^}]*\}\s*from '@tauri-apps\/api\/core'/);
    assert.ok(appSource.includes('@activate="checkOrInstallSoftwareUpdate"'));
    assert.ok(updateAction.includes('softwareUpdateAvailable.value'));
    assert.ok(updateAction.includes('await installSoftwareUpdate()'));
    assert.ok(updateAction.includes('await checkSoftwareUpdate()'));
    assert.ok(installAction.includes('const onEvent = new Channel()'));
    assert.ok(installAction.includes("invoke('install_software_update_and_restart'"));
    assert.ok(installAction.includes('expectedVersion: update.version'));
    assert.ok(installAction.includes('Restarting Dust Wave Social'));
    assert.equal(installAction.includes('downloadAndInstall'), false);
    assert.ok(updateStatusButtonSource.includes('data-icon="arrow-down-circle"'));
    assert.ok(updateStatusButtonSource.includes("return props.available ? 'Install' : 'Update'"));
    assert.ok(updateStatusButtonSource.includes(':aria-label="actionDescription"'));
    assert.ok(updateStatusButtonSource.includes(':aria-busy="busy"'));
});

test('Tauri updater resources stay outside Vue deep-reactivity proxies', () => {
    assert.match(appSource, /import\s*\{[^}]*\bshallowRef\b[^}]*\}\s*from 'vue'/);
    assert.ok(appSource.includes('const softwareUpdateAvailable = shallowRef(null)'));
    assert.equal(appSource.includes('const softwareUpdateAvailable = ref(null)'), false);

    const update = new Update({
        rid: 7,
        currentVersion: '0.1.2',
        version: '0.1.3',
        date: null,
        body: '',
        rawJson: {},
    });
    const available = shallowRef(update);

    assert.equal(isProxy(available.value), false);
    assert.equal(available.value.rid, 7);
});
