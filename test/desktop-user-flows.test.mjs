import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

async function projectFile(...parts) {
    return readFile(path.join(projectRoot, ...parts), 'utf8');
}

const sources = {
    app: await projectFile('resources', 'desktop', 'src', 'App.vue'),
    styles: await projectFile('resources', 'desktop', 'src', 'styles.css'),
    backup: await projectFile('resources', 'desktop', 'src', 'components', 'BackupRestorePanel.vue'),
    updates: await projectFile('resources', 'desktop', 'src', 'components', 'SoftwareUpdatesPanel.vue'),
    updateButton: await projectFile('resources', 'desktop', 'src', 'components', 'UpdateStatusButton.vue'),
    confirm: await projectFile('resources', 'desktop', 'src', 'components', 'ConfirmDialog.vue'),
    contextualEditor: await projectFile('resources', 'desktop', 'src', 'components', 'ContextualEditor.vue'),
    tabs: await projectFile('resources', 'desktop', 'src', 'components', 'WorkspaceTabs.vue'),
    postDetail: await projectFile('resources', 'desktop', 'src', 'components', 'PostDetailModal.vue'),
    preview: await projectFile('resources', 'desktop', 'src', 'components', 'ProviderPreviewCard.vue'),
};
const flowDocument = await projectFile('docs', 'USER_FLOWS.md');

const flowContracts = [
    {
        id: 'CORE-01',
        name: 'launch and load the local workspace',
        markers: [['app', 'const load = async () =>'], ['app', 'v-if="loadError" class="error-panel"'], ['app', 'onMounted(async () =>']],
    },
    {
        id: 'CORE-02',
        name: 'navigate every primary work area',
        markers: [['app', 'const navigationGroups = ['], ['app', "viewIds: ['connections', 'reports', 'tags']"], ['app', ':aria-current="activeView === view.id ? \'page\' : undefined"'], ['app', '@click="activeView = view.id"']],
        absent: [['app', "id: 'accounts',\n        label: 'Accounts'"], ['app', "id: 'services',\n        label: 'Services'"], ['app', "id: 'profile',\n        label: 'Profile'"]],
    },
    {
        id: 'DASH-01',
        name: 'review dashboard summaries and workspace attention',
        markers: [['app', 'const attentionNotices = computed(() =>'], ['app', 'for (const issue of health.value?.issues || [])'], ['app', '<section v-if="attentionNotices.length"']],
    },
    {
        id: 'DASH-02',
        name: 'switch dashboard account analytics and reporting period',
        markers: [['app', '@click="loadReport(account.id)"'], ['app', 'role="tab"'], ['app', '@click="reportPeriod = period; loadReport()"']],
    },
    {
        id: 'POST-01',
        name: 'compose and save a draft',
        markers: [['app', 'const saveDraftPost = async () =>'], ['app', '<form class="draft-form" @submit.prevent="saveDraftPost">'], ['app', "editingPostUuid.value ? 'Update Post' : 'Save Draft'"]],
    },
    {
        id: 'POST-02',
        name: 'target accounts and edit account-specific versions',
        markers: [['app', 'draftAccountIds'], ['app', 'draftAccountBodies'], ['app', 'composer-version-tab']],
    },
    {
        id: 'POST-03',
        name: 'add labels emoji and media to a draft',
        markers: [['app', 'draftTagIds'], ['app', '<EmojiPickerPanel @select="insertDraftEmoji"'], ['app', 'draftMediaIds']],
    },
    {
        id: 'POST-04',
        name: 'autosave recover and discard composer state',
        markers: [['app', "const DRAFT_STORAGE_KEY = 'dust-wave-social-composer-draft'"], ['app', 'const restoreComposerDraft = () =>'], ['app', 'window.localStorage.setItem(DRAFT_STORAGE_KEY']],
    },
    {
        id: 'POST-05',
        name: 'validate content and review provider previews',
        markers: [['app', 'const validatePost = async (uuid) =>'], ['app', '<ProviderPreviewCard'], ['preview', 'preview.providerKey']],
    },
    {
        id: 'POST-06',
        name: 'schedule a draft for later',
        markers: [['app', 'const scheduleCurrentDraft = async'], ['app', 'scheduled_at: postNow ? new Date().toISOString()'], ['app', 'canScheduleDraft']],
    },
    {
        id: 'POST-07',
        name: 'confirm and publish a post now',
        markers: [['app', 'postNowConfirmationOpen'], ['app', 'aria-labelledby="post-now-title"'], ['app', 'Publish this post now to the selected social accounts?']],
    },
    {
        id: 'POST-08',
        name: 'browse filter select and paginate posts',
        markers: [['app', 'const postPageEnd = computed(() =>'], ['app', 'if (!postQuery.value.total || !postQuery.value.items?.length)'], ['app', 'aria-label="Post status"'], ['app', 'pagination-controls']],
    },
    {
        id: 'POST-09',
        name: 'view edit duplicate validate and delete posts',
        markers: [['app', 'const openPostDetail = async (post) =>'], ['app', 'const editSelectedPostFromDetail = async () =>'], ['app', 'const duplicatePost = async (uuid) =>'], ['app', 'any queued publishing work for it will be cancelled.'], ['app', "confirmLabel: 'Delete post'"], ['postDetail', 'Edit in composer'], ['postDetail', "defineEmits(['close', 'edit'])"]],
    },
    {
        id: 'POST-10',
        name: 'retry failed posts safely',
        markers: [['app', 'const retryFailedPostNow = async (post) =>'], ['app', 'const openPostScheduleEditor = (post) =>'], ['app', 'may publish immediately to every account assigned to it.'], ['app', "confirmLabel: 'Retry now'"], ['app', "post.status === 'failed' ? 'Retry later' : post.scheduled_at ? 'Reschedule' : 'Schedule'"], ['app', 'class="post-schedule-editor"']],
    },
    {
        id: 'CAL-01',
        name: 'navigate calendar dates and ranges',
        markers: [['app', 'const moveCalendar = async'], ['app', 'aria-label="Calendar range"'], ['app', ':aria-selected="postFilter.calendar_type === \'month\'"']],
    },
    {
        id: 'CAL-02',
        name: 'filter the calendar',
        markers: [['app', 'class="post-query-form is-calendar-filter"'], ['app', 'aria-label="Calendar date"'], ['app', 'aria-label="Search post content"']],
        absent: [['app', '<select v-model="postFilter.calendar_type">']],
    },
    {
        id: 'CAL-03',
        name: 'start a post from a calendar date or time slot',
        markers: [['app', 'const createPostFromCalendarDate = (date) =>'], ['app', 'const createPostFromCalendarSlot = (date, hour) =>']],
    },
    {
        id: 'CAL-04',
        name: 'review edit and bulk-delete calendar posts',
        markers: [['app', '@click="openPostDetail(post)"'], ['app', 'const editSelectedPostFromDetail = async () =>'], ['app', 'contextualEditOrigin.value = originView'], ['app', 'const bulkDeletePosts = async () =>'], ['app', 'their queued publishing work will be cancelled.'], ['app', "confirmLabel: 'Delete selected posts'"]],
    },
    {
        id: 'MEDIA-01',
        name: 'import local media by picker path or drop',
        markers: [['app', 'const importMediaFile = async () =>'], ['app', '@drop.prevent="handleMediaDrop"'], ['app', 'aria-label="Local media file path"']],
    },
    {
        id: 'MEDIA-02',
        name: 'download media from a URL',
        markers: [['app', 'const downloadExternalMedia = async () =>'], ['app', 'aria-label="Media URL"'], ['app', 'aria-label="Downloaded media source label"']],
    },
    {
        id: 'MEDIA-03',
        name: 'browse select clean and delete uploaded media',
        markers: [['app', 'aria-label="Search uploaded media"'], ['app', 'const deleteSelectedMedia = async () =>'], ['app', 'Original files outside Dust Wave Social will not be changed.'], ['app', "confirmLabel: 'Delete selected media'"]],
    },
    {
        id: 'MEDIA-04',
        name: 'create a post from selected media',
        markers: [['app', 'const createPostFromSelectedMedia = async () =>'], ['app', '@click="createPostFromSelectedMedia"'], ['app', 'selectedExternalMediaPolicyNote']],
    },
    {
        id: 'MEDIA-05',
        name: 'search and download stock media',
        markers: [['app', "id: 'unsplash'"], ['app', 'const searchExternalMedia = async'], ['app', 'canDownloadExternalMediaItem(item)']],
    },
    {
        id: 'MEDIA-06',
        name: 'search and attach GIF references',
        markers: [['app', "id: 'klipy'"], ['app', "activeMediaTab === 'gifs' ? 'Search KLIPY'"], ['app', 'Attach only']],
    },
    {
        id: 'MEDIA-07',
        name: 'run and review local AI media tools',
        markers: [['app', 'const probeLocalAiRuntime = async () =>'], ['app', 'const preflightLocalAiMedia = async (item) =>'], ['app', 'const upscaleLocalAiMedia = async'], ['app', 'cancelLocalAiOperation']],
    },
    {
        id: 'ACCT-01',
        name: 'start account onboarding and copy intake material',
        markers: [['app', 'const copyAccountOnboardingTemplate = async () =>'], ['app', 'const accountProviderChoices = ['], ['app', 'v-if="!activeAccountProvider" class="account-provider-choice-list"'], ['app', 'aria-labelledby="add-account-title"'], ['styles', '.account-provider-choice-list {']],
    },
    {
        id: 'ACCT-02',
        name: 'connect an X account',
        markers: [['app', 'const startTwitterOAuth = async () =>'], ['app', 'const connectTwitterAccount = async () =>'], ['app', 'Authorization code is required']],
    },
    {
        id: 'ACCT-03',
        name: 'connect Facebook Pages and Instagram accounts',
        markers: [['app', 'const startFacebookOAuth = async () =>'], ['app', 'const exchangeFacebookOAuth = async () =>'], ['app', 'const connectFacebookPages = async () =>'], ['app', 'const connectInstagramAccounts = async () =>']],
    },
    {
        id: 'ACCT-04',
        name: 'register and connect a Mastodon account',
        markers: [['app', 'const registerMastodonApp = async () =>'], ['app', 'const connectMastodonAccount = async () =>'], ['app', 'Mastodon server is required']],
    },
    {
        id: 'ACCT-05',
        name: 'connect TikTok through the broker',
        markers: [['app', 'const openTikTokBrokerAuth = async () =>'], ['app', 'const connectTikTokAccount = async () =>'], ['app', 'Broker connection credential is required']],
    },
    {
        id: 'ACCT-06',
        name: 'refresh import queue and disconnect accounts',
        markers: [['app', 'const accountProviderOperations = {'], ['app', 'const refreshAccount = async (account) =>'], ['app', 'const importAccountData = async (account) =>'], ['app', 'const queueAllAccountImports = async () =>'], ['app', "title: 'Disconnect this account?'"], ['app', "confirmLabel: 'Disconnect account'"]],
    },
    {
        id: 'SVC-01',
        name: 'configure and save provider services',
        markers: [['app', 'const saveServiceSettings = async'], ['app', "invoke('save_service_credential'"], ['app', "invoke('save_service'"], ['app', 'const activeServiceCredentialSummary = computed'], ['app', 'Edit Settings'], ['app', 'role="status" aria-live="polite"']],
    },
    {
        id: 'SVC-02',
        name: 'open and copy provider setup without secrets',
        markers: [['app', 'const copyServiceSetupField = async'], ['app', 'const copyProviderSetupBundle = async'], ['app', '<summary>Share setup</summary>'], ['app', 'Provider setup packet copied without secret values.']],
    },
    {
        id: 'SVC-03',
        name: 'pair this Mac for Instagram local-image publishing',
        markers: [['app', 'const enrollMediaStaging = async () =>'], ['app', "invoke('enroll_media_staging'"], ['app', 'autocomplete="one-time-code"'], ['app', 'This Mac is paired'], ['app', 'Advanced Manual Setup'], ['app', 'The setup code cannot be reused.']],
    },
    {
        id: 'RPT-01',
        name: 'select and review provider reports',
        markers: [['app', '<h2>Analytics</h2>'], ['app', 'aria-label="Report account"'], ['app', 'Connect an account first'], ['app', "@click=\"activeView = 'connections'; activeConnectionTab = 'accounts'; openAddAccountModal()\""], ['app', 'const loadReport = async']],
    },
    {
        id: 'TAG-01',
        name: 'create edit assign filter and delete tags',
        markers: [['app', 'const createTag = async () =>'], ['app', 'const updateTag = async (uuid) =>'], ['app', 'title="Edit label"'], ['app', 'Posts will remain, but this label will be removed from every post that currently uses it.'], ['app', 'aria-label="Label color"'], ['contextualEditor', '@keydown.esc.stop.prevent="cancel"']],
    },
    {
        id: 'SET-01',
        name: 'configure and test desktop notifications',
        markers: [['app', 'desktop_notifications'], ['app', 'const sendTestNotification = async () =>'], ['app', 'Test notification sent']],
    },
    {
        id: 'SET-02',
        name: 'opt into Local AI Media Labs',
        markers: [['app', 'local_ai_media_labs'], ['app', "'Local AI on' : 'Local AI off'"], ['app', 'v-if="localAiLabsEnabled"']],
    },
    {
        id: 'SET-03',
        name: 'save time display and default-account settings',
        markers: [['app', 'const saveSettings = async () =>'], ['app', 'settingsDraft.timezone'], ['app', 'settingsDraft.week_starts_on'], ['app', 'settingsDraft.default_accounts']],
    },
    {
        id: 'PROF-01',
        name: 'save operator profile and understand local security',
        markers: [['app', '<form class="settings-panel-stack" @submit.prevent="saveSettings">'], ['app', '<h3>Local identity</h3>'], ['app', 'Dust Wave Social does not provide an app-specific password.'], ['app', 'does not create a server login session']],
    },
    {
        id: 'SYS-01',
        name: 'review actionable system health',
        markers: [['app', 'System Health'], ['app', 'health.issues'], ['app', "['Missing active service credentials', 'Instagram local media setup needed'].includes(issue.title)"], ['app', "service: issue.title === 'Instagram local media setup needed' ? 'media_staging' : undefined"]],
    },
    {
        id: 'SYS-02',
        name: 'run confirmed maintenance and recovery actions',
        markers: [['app', 'const clearResolvedSystemState = async () =>'], ['app', "title: 'Clear resolved system state?'"], ['app', 'Completed and cancelled background work plus expired provider limits will be removed.'], ['app', 'const recoverStaleProcessingJobs = async () =>'], ['app', 'const retryFailedAccountImports = async () =>']],
    },
    {
        id: 'SYS-03',
        name: 'copy support state and manage redacted logs',
        markers: [['app', 'const copySystemStatus = async () =>'], ['app', 'const copyAppDataPath = async () =>'], ['app', 'const exportSystemLog = async () =>'], ['app', "title: 'Clear local system logs?'"], ['app', "confirmLabel: 'Clear logs'"]],
    },
    {
        id: 'SYS-04',
        name: 'create a local backup',
        markers: [['app', 'const createLocalBackup = async () =>'], ['backup', 'OS keychain secrets are not included.'], ['backup', 'Create Backup']],
    },
    {
        id: 'SYS-05',
        name: 'choose and restore a backup safely',
        markers: [['app', 'const restoreLocalBackup = async () =>'], ['app', 'Dust Wave Social will create a safety backup, replace current local data with the selected backup, and then reload the workspace.'], ['app', "confirmLabel: 'Restore backup'"], ['backup', ':disabled="restoreRunning || backupRunning || !restorePath.trim()"']],
    },
    {
        id: 'SYS-06',
        name: 'check install verify and restart software updates',
        markers: [['app', 'const checkOrInstallSoftwareUpdate = async () =>'], ['app', "invoke('install_software_update_and_restart'"], ['updates', 'aria-live="polite"'], ['updateButton', ':aria-busy="busy"']],
    },
    {
        id: 'A11Y-01',
        name: 'expose navigation tabs dialogs fields feedback and intent accessibly',
        markers: [['app', ':aria-current="activeView === view.id ? \'page\' : undefined"'], ['confirm', 'role="dialog"'], ['confirm', 'aria-modal="true"'], ['confirm', '@keydown.esc.prevent="emit(\'cancel\')"'], ['contextualEditor', ':aria-label="title"'], ['contextualEditor', ':aria-busy="busy"'], ['contextualEditor', 'returnFocusTarget.focus()'], ['tabs', 'role="tablist"'], ['tabs', ':aria-selected="modelValue === tab.id"'], ['app', ':aria-selected="activeServiceTab === service.id"'], ['app', ':aria-selected="activeMediaTab === tab.id"'], ['app', 'aria-label="Report period"']],
        absent: [['app', 'window.confirm']],
    },
];

test('canonical user-flow document and regression registry remain one-to-one', () => {
    const documentedIds = [...flowDocument.matchAll(/^\| ([A-Z0-9]+-\d+) \|/gm)].map((match) => match[1]);
    const contractIds = flowContracts.map((flow) => flow.id);

    assert.deepEqual([...new Set(documentedIds)].sort(), [...contractIds].sort());
    assert.equal(new Set(documentedIds).size, documentedIds.length, 'User-flow document contains duplicate IDs');
    assert.equal(new Set(contractIds).size, contractIds.length, 'Regression registry contains duplicate IDs');
});

for (const flow of flowContracts) {
    test(`${flow.id}: ${flow.name}`, () => {
        assert.ok(flowDocument.includes(`| ${flow.id} |`), `${flow.id} is missing from docs/USER_FLOWS.md`);

        for (const [sourceName, marker] of flow.markers) {
            assert.ok(
                sources[sourceName].includes(marker),
                `${flow.id} is missing ${sourceName} marker: ${marker}`,
            );
        }

        for (const [sourceName, marker] of flow.absent || []) {
            assert.equal(
                sources[sourceName].includes(marker),
                false,
                `${flow.id} restored confusing ${sourceName} marker: ${marker}`,
            );
        }
    });
}
