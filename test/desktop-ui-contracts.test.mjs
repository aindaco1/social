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
const providerSetupSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'providerSetup.js'),
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
const backupRestoreSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'BackupRestorePanel.vue'),
    'utf8',
);
const commandsSource = await readFile(
    path.join(projectRoot, 'src-tauri', 'src', 'commands.rs'),
    'utf8',
);
const providerPreviewSource = await readFile(
    path.join(projectRoot, 'resources', 'desktop', 'src', 'components', 'ProviderPreviewCard.vue'),
    'utf8',
);
const desktopConfig = JSON.parse(await readFile(path.join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const desktopCapability = JSON.parse(await readFile(path.join(projectRoot, 'src-tauri', 'capabilities', 'default.json'), 'utf8'));
const nativeStartupSource = await readFile(path.join(projectRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8');
const localAiRuntimeSource = await readFile(path.join(projectRoot, 'resources/desktop/src/localAiModelRuntime.js'), 'utf8');
const localAiWorkerSource = await readFile(path.join(projectRoot, 'resources/desktop/src/localAi.worker.js'), 'utf8');

test('credential namespace is initialized from the running bundle before database startup', () => {
    const initialize = nativeStartupSource.indexOf('secrets::initialize_namespace(&app.config().identifier)?');
    assert.ok(initialize >= 0);
    assert.ok(initialize < nativeStartupSource.indexOf('db::Database::initialize(app.handle())'));
});

test('packaged local AI permits bundled WebAssembly without enabling JavaScript eval or remote scripts', () => {
    const directives = Object.fromEntries(desktopConfig.app.security.csp.split(';').map((directive) => {
        const [name, ...sources] = directive.trim().split(/\s+/);
        return [name, sources];
    }));
    assert.deepEqual(directives['script-src'], ["'self'", "'wasm-unsafe-eval'"]);
    assert.deepEqual(directives['worker-src'], ["'self'"]);
    assert.deepEqual(directives['connect-src'], ["'self'", 'ipc:', 'http://ipc.localhost']);
    const loader = sourceBetween('const loadLocalAiImage', 'const prepareLocalAiSourceCanvas');
    assert.ok(loader.indexOf("image.crossOrigin = 'anonymous'") >= 0);
    assert.ok(loader.indexOf("image.crossOrigin = 'anonymous'") < loader.indexOf('image.src = url'));
    assert.deepEqual(desktopConfig.app.security.assetProtocol.scope, ['$APPDATA/media/**']);
});

test('GPU inference requires supported JSPI readback and otherwise reuses the Wasm CPU path', () => {
    const runtime = localAiRuntimeSource;
    assert.ok(runtime.includes("['jspi', 'relaxedSimd', 'threads'].map((feature) => litert.supportsFeature(feature))"));
    assert.ok(runtime.includes("loadLiteRt(new URL('wasm/', assetsBase).href, options.loadOptions)"));
    assert.ok(runtime.includes('const gpuReadback = Boolean(webgpu && jspi && relaxedSimd)'));
    assert.ok(runtime.includes("accelerator: gpuReadback ? ['webgpu', 'wasm'] : 'wasm'"));
    assert.ok(appSource.includes('using compatible Wasm CPU processing'));
    const drawing = sourceBetween('const drawLocalAiUpscaleTile', 'const upscaleLocalAiMediaWithModel');
    assert.ok(drawing.includes('localAiOutputRgba(outputData)'));
});

test('media file and image work leaves the native event loop through one blocking-task boundary', () => {
    for (const command of ['import_media_file', 'download_external_media', 'local_ai_preflight_media',
        'create_local_ai_upscale_derivative', 'save_local_ai_model_upscale_derivative',
        'create_local_ai_crop_derivative', 'local_ai_media_search', 'draft_local_ai_alt_text']) {
        const body = sourceBetweenText(commandsSource, `pub async fn ${command}(`, '\n}');
        assert.ok(body.includes('run_media_task(database, move |db|'), command);
        assert.ok(body.includes('.await'), command);
    }
    const runner = sourceBetweenText(commandsSource, 'async fn run_media_task', '\n}');
    assert.ok(runner.includes('database.inner().clone()'));
    assert.ok(runner.includes('tauri::async_runtime::spawn_blocking(move ||'));
});

test('native zoom reuses Tauri hotkeys with only the required webview permission', () => {
    assert.equal(desktopConfig.app.windows[0].zoomHotkeysEnabled, true);
    assert.ok(desktopCapability.permissions.includes('core:webview:allow-set-webview-zoom'));
    assert.ok(appSource.includes('⌘0 for actual size'));
    const compactStyles = desktopStylesSource.slice(desktopStylesSource.lastIndexOf('@media (max-width: 900px)'));
    assert.match(compactStyles, /\.composer-action-bar\s*\{\s*position: static;/);
});

test('unpublished previews do not invent engagement counts or publication age', () => {
    assert.doesNotMatch(providerPreviewSource, />\s*(?:116|27|312|19h|0 comments)\s*</);
    assert.ok(providerPreviewSource.includes('class="provider-preview-twitter-metrics" aria-hidden="true"'));
    assert.ok(appSource.includes("`${selectedDraftAccounts.length} account${selectedDraftAccounts.length === 1 ? '' : 's'} selected`"));
});

test('editor, selection controls, and restore feedback expose accessible names and states', () => {
    assert.ok(appSource.includes("'aria-label': 'Post content'"));
    assert.ok(appSource.includes("'aria-multiline': 'true'"));
    assert.ok(appSource.includes('aria-label="Select all posts on this page"'));
    assert.ok(appSource.includes(':aria-label="`Select post: ${post.preview || post.uuid}`"'));
    assert.ok(appSource.includes(':aria-label="`Select media: ${item.name || item.id}`"'));
    assert.ok(appSource.includes(':aria-label="`Select media: ${item.name || item.uuid}`"'));
    assert.ok(backupRestoreSource.includes('aria-label="Backup folder path"'));
    assert.ok(backupRestoreSource.includes(':aria-busy="backupRunning || restoreRunning"'));
    assert.match(backupRestoreSource, /v-if="backupError"[^>]*role="alert"/);
    assert.match(backupRestoreSource, /v-if="restoreError"[^>]*role="alert"/);
    const restore = sourceBetween('const restoreLocalBackup', 'const clearResolvedSystemState');
    assert.ok(restore.includes('resetDraftEditor()'));
    assert.equal(restore.includes('localStorage.removeItem'), false);
});

test('emoji selection and dismissal restore focus without leaking Enter or trapping Escape', () => {
    const close = sourceBetween('const closeEmojiPicker', 'const insertDraftEmoji');
    const insert = sourceBetween('const insertDraftEmoji', 'const canEditPost');
    assert.ok(close.indexOf('await nextTick()') > close.indexOf('emojiPickerOpen.value = false'));
    assert.ok(close.indexOf('emojiPickerButton.value?.focus()') > close.indexOf('await nextTick()'));
    assert.ok(insert.indexOf('editor.view.focus()') > insert.indexOf('await closeEmojiPicker({ returnFocus: false })'));
    assert.ok(insert.indexOf('editor.view.focus()') > insert.indexOf('editor.commands.insertContent(value)'));
    assert.ok(appSource.includes('@keydown.esc.capture.stop.prevent="closeEmojiPicker()"'));
    assert.ok(appSource.includes('@keydown.enter.capture.prevent'));
    assert.ok(appSource.includes('aria-controls="composer-emoji-picker"'));
});

test('system health runs blocking diagnostics off the UI thread and displays the actual tool result', () => {
    const healthCommand = sourceBetweenText(commandsSource, 'pub async fn system_health(', 'pub fn dashboard_summary(');
    assert.ok(healthCommand.includes('tauri::async_runtime::spawn_blocking(move ||'));
    assert.ok(appSource.includes("...['ffmpeg', 'ffprobe'].map((tool) => ({"));
    assert.ok(appSource.includes("health.value?.media_tools?.[tool]?.detail || 'Checking availability…'"));
    assert.ok(appSource.includes('v-else-if="!workspaceReady"'));
    assert.ok(appSource.includes('Loading your local workspace…'));
    assert.ok(appSource.includes("if (!workspaceReady.value) return 'Loading…'"));
    const startup = sourceBetween('onMounted(async () =>', 'onUnmounted(() =>');
    assert.ok(startup.indexOf('workspaceReady.value = true') > startup.indexOf('restoreComposerDraft()'));
});

function sourceBetween(start, end) {
    return sourceBetweenText(appSource, start, end);
}

function sourceBetweenText(source, start, end) {
    const startIndex = source.indexOf(start);
    const endIndex = source.indexOf(end, startIndex + start.length);

    assert.notEqual(startIndex, -1, `Missing source marker: ${start}`);
    assert.notEqual(endIndex, -1, `Missing source marker: ${end}`);

    return source.slice(startIndex, endIndex);
}

test('empty account actions and native dropdowns use shared compact layout contracts', () => {
    assert.match(appSource, /<div v-else class="empty-action-row">\s*<span>No connected accounts\.<\/span>\s*<button[^>]*openAddAccountModal\(\)/);
    assert.match(desktopStylesSource, /\.empty-action-row\s*{[^}]*flex-wrap: wrap;[^}]*gap: 12px 16px;/s);
    assert.match(desktopStylesSource, /\nselect\s*{[^}]*width: min\(100%, var\(--select-width, 100%\)\);[^}]*font-size: 13px;[^}]*font-weight: 500;/s);
    assert.match(appSource, /class="select-wide" aria-label="Timezone"/);
    assert.match(appSource, /class="select-compact" aria-label="Date format"/);
    assert.match(appSource, /class="period-select select-wide"/);
    assert.ok(appSource.includes("['UTC', ...supportedTimezones, settingsDraft.value.timezone]"));
    assert.equal(desktopStylesSource.includes('.report-selects .period-select:first-child'), false);
});

test('every tab group uses the shared keyboard handler and calendar selections are named', () => {
    for (const source of [appSource, workspaceTabsSource]) {
        const groups = source.match(/<[^>]+role="tablist"[^>]*>/g) || [];
        assert.ok(groups.length);
        for (const group of groups) assert.ok(group.includes('v-tab-navigation'), group);
    }
    assert.ok(appSource.includes(':aria-label="`Select post: ${post.preview || post.uuid}`"'));
    assert.match(appSource, /watch\(activeView, async \(view\) => {\s*window.scrollTo\(0, 0\);/);
    assert.ok(appSource.includes('for (const post of calendarAgenda.value.items)'));
    assert.ok(appSource.includes('calendarAgenda.page === calendarAgenda.pages'));
});

test('TikTok service setup defaults to the deployed Dust Wave broker', () => {
    const match = providerSetupSource.match(/const dustWaveTikTokBrokerUrl = '([^']+)'/);

    assert.ok(match, 'Dust Wave TikTok broker URL constant is missing');
    assert.equal(match[1], 'https://dustwave-tiktok-broker.jogo.workers.dev');
    assert.ok(providerSetupSource.includes("value: `${dustWaveTikTokBrokerUrl}/api/tiktok/oauth/callback`"));
    assert.ok(providerSetupSource.includes('defaultValue: dustWaveTikTokBrokerUrl'));
    assert.ok(providerSetupSource.includes('placeholder: dustWaveTikTokBrokerUrl'));
});

test('TikTok setup models one domain verification and the broker as a Web redirect', () => {
    const tiktokService = sourceBetweenText(providerSetupSource, "id: 'tiktok'", "id: 'unsplash'");

    assert.ok(providerSetupSource.includes("const dustWaveTikTokPlatforms = 'Web and Desktop'"));
    assert.ok(providerSetupSource.includes('Verify dustwave.xyz once with a DNS TXT record'));
    assert.ok(providerSetupSource.includes('Display API is not a separate product in the current portal'));
    assert.ok(providerSetupSource.includes('Production > Import > Import from Sandbox'));
    assert.ok(tiktokService.includes("label: 'Login Kit Web redirect URI'"));
    assert.equal(tiktokService.includes("label: 'Broker OAuth callback URL'"), false);
});

test('TikTok desktop credentials exclude the client secret', () => {
    const tiktokService = sourceBetweenText(providerSetupSource, "id: 'tiktok'", "id: 'unsplash'");

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
    assert.ok(confirmDialogSource.includes('@keydown.esc.stop.prevent="!busy && emit(\'cancel\')"'));
    assert.ok(confirmDialogSource.includes('v-dialog-focus'));
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
    const mediaStagingService = sourceBetweenText(providerSetupSource, "id: 'media_staging'", "id: 'twitter'");

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
    const serviceDefinitionsSource = sourceBetweenText(providerSetupSource, 'export const serviceDefinitions = [', '\n];');
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

test('provider setup uses the shared catalog and one guided portal-to-account path', () => {
    assert.ok(appSource.includes("from '@desktop/providerSetup.js'"));
    assert.ok(appSource.includes('const activeServiceSetupSteps = computed(() =>'));
    assert.ok(appSource.includes('Complete these in order.'));
    assert.ok(appSource.includes('Copy Exact Setup'));
    assert.ok(appSource.includes('connectActiveServiceAccount'));
    assert.ok(appSource.includes("activeConnectionTab.value = 'accounts';\n        openAddAccountModal(provider);"));
    assert.ok(appSource.includes('verifyActiveMediaService'));
    assert.ok(providerSetupSource.includes("'pages_show_list',\n    'pages_read_engagement',\n    'read_insights',"));
    assert.ok(providerSetupSource.includes("accountProviderKeys: ['facebook_page', 'instagram']"));
    assert.ok(providerSetupSource.includes("verificationTab: 'stock'"));
    assert.ok(providerSetupSource.includes("verificationTab: 'gifs'"));
});

test('local AI media labs are opt-in and use bundled LiteRT assets', () => {
    assert.ok(appSource.includes('local_ai_media_labs'));
    assert.ok(localAiWorkerSource.includes("import('@litertjs/core')"));
    assert.equal(appSource.includes("import('@litertjs/core')"), false);
    assert.ok(localAiRuntimeSource.includes("await litert.loadLiteRt(new URL('wasm/', assetsBase).href, options.loadOptions)"));
    assert.ok(localAiRuntimeSource.includes("fetchAsset(new URL('models/manifest.json', assetsBase).href"));
    assert.ok(localAiRuntimeSource.includes("loadAndCompile(new URL(`models/${bundle.modelFile.path}`, assetsBase).href"));
    assert.ok(localAiRuntimeSource.includes('new Tensor(inputBytes, inputShape)'));
    assert.ok(appSource.includes("toDataURL('image/png')"));
    assert.ok(appSource.includes('compiled_model'));
    assert.ok(appSource.includes("invoke('save_local_ai_model_upscale_derivative'"));
    assert.ok(appSource.includes('cancelLocalAiOperation'));
    assert.ok(appSource.includes("invoke('local_ai_media_search'"));
    assert.ok(appSource.includes('Search filenames or image properties'));
    assert.equal(appSource.includes('Semantic media search'), false);
});

test('model probe and inference share a cancellable classic worker with operation-owned cleanup', () => {
    assert.ok(appSource.includes("new Worker(new URL('./localAi.worker.js', import.meta.url))"));
    assert.ok(localAiWorkerSource.includes("import('./localAiModelRuntime.js')"));
    assert.ok(localAiWorkerSource.includes("base.pathname !== '/litert/'"));
    for (const [start, end] of [['const upscaleLocalAiMediaWithModel', 'const probeLocalAiRuntime'], ['const probeLocalAiRuntime', 'const cancelLocalAiOperation']]) {
        const body = sourceBetween(start, end);
        assert.ok(body.includes('openLocalAiModelSession(signal)'));
        assert.ok(body.includes('recordLocalAiModelReady(modelBundle)'));
        assert.ok(body.includes("client.request('release')"));
        assert.ok(body.includes('client?.dispose()'));
    }
    const model = sourceBetween('const upscaleLocalAiMediaWithModel', 'const probeLocalAiRuntime');
    assert.ok(model.includes("client.request('tile', { inputBytes }, [inputBytes.buffer])"));
    assert.ok(model.includes('observeLocalAiResponsiveness()'));
    assert.ok(sourceBetween('const probeLocalAiRuntime', 'const cancelLocalAiOperation').includes('{ cancellable: true }'));
});

test('local media feedback shares live regions, editable drafts, and the model commit boundary', async () => {
    const feedback = await readFile(path.join(projectRoot, 'resources/desktop/src/components/LocalMediaFeedback.vue'), 'utf8');
    assert.ok(feedback.includes('role="status" aria-live="polite" aria-atomic="true"'));
    assert.ok(feedback.includes('role="alert" aria-atomic="true"'));
    assert.ok(feedback.includes('v-if="canCancel"'));
    assert.ok(feedback.includes('aria-describedby="local-ai-alt-text-review"'));
    assert.ok(feedback.includes("dismissControl('update:draft', null)"));
    assert.ok(feedback.includes('feedback.value?.focus({ preventScroll: true })'));
    assert.ok(appSource.includes('v-model:draft="localAiAltTextDraft"'));
    const altText = sourceBetween('const draftLocalAiAltText', 'const searchLocalAiMedia');
    assert.ok(altText.includes('currentDraft.text !== currentDraft.generatedText'));
    assert.ok(altText.includes("cancelLabel: 'Keep draft'"));
    assert.ok(altText.indexOf('await requestConfirmation') < altText.indexOf("invoke('draft_local_ai_alt_text'"));
    assert.ok(appSource.includes('localAiSearchResults.matches.slice(0, LOCAL_MEDIA_SEARCH_PREVIEW_LIMIT)'));
    const model = sourceBetween('const upscaleLocalAiMediaWithModel', 'const probeLocalAiRuntime');
    assert.ok(model.includes('beginCommit()'));
    assert.ok(model.indexOf('beginCommit()') < model.indexOf("invoke('save_local_ai_model_upscale_derivative'"));
    assert.ok(appSource.includes('{ reload: true, cancellable: true }'));
    assert.equal(appSource.includes('localAiAbortController'), false);
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

test('launch update checks stay quiet and reuse the manual signed-update path', () => {
    const updateCheck = sourceBetween('const checkSoftwareUpdate', 'const installSoftwareUpdate');
    const mountedLifecycle = sourceBetween('onMounted(async () =>', 'onUnmounted(() =>');

    assert.ok(updateCheck.includes('async ({ silent = false } = {})'));
    assert.ok(updateCheck.includes("await checkForUpdate({ timeout: 15000 })"));
    assert.ok(updateCheck.includes('softwareUpdateLastCheckWasAutomatic.value = silent'));
    assert.ok(updateCheck.includes('softwareUpdateLastCheckWasAutomatic.value = false'));
    assert.ok(updateCheck.includes("softwareUpdateStatus.value = softwareUpdateLastCheckWasAutomatic.value"));
    assert.ok(updateCheck.includes("? 'Automatic update check unavailable'"));
    assert.equal((mountedLifecycle.match(/checkSoftwareUpdate/g) || []).length, 1);
    assert.ok(mountedLifecycle.includes('void checkSoftwareUpdate({ silent: true })'));
    assert.ok(appSource.includes(':checking="softwareUpdateCheckingVisible"'));
    assert.ok(appSource.includes(':status="softwareUpdateTopbarStatus"'));
    assert.ok(appSource.includes(':error="softwareUpdateTopbarError"'));
    assert.equal(mountedLifecycle.includes('installSoftwareUpdate'), false);
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
