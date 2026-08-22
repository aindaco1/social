<script setup>
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { openUrl } from '@tauri-apps/plugin-opener';
import { check as checkForUpdate } from '@tauri-apps/plugin-updater';
import Document from '@tiptap/extension-document';
import History from '@tiptap/extension-history';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import Text from '@tiptap/extension-text';
import Typography from '@tiptap/extension-typography';
import { EditorContent, useEditor as useTipTapEditor } from '@tiptap/vue-3';
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue';
import { COLOR_PALLET_LIST } from '@/Constants/ColorPallet';
import Div from '@/Extensions/TipTap/Div';
import dustWaveSquareLogoUrl from '@desktop/assets/dust-wave-square.png';
import ConfirmDialog from '@desktop/components/ConfirmDialog.vue';
import ContextualEditor from '@desktop/components/ContextualEditor.vue';
import UpdateStatusButton from '@desktop/components/UpdateStatusButton.vue';
import WorkspaceTabs from '@desktop/components/WorkspaceTabs.vue';

const WORKER_POLL_MS = 60 * 1000;
const MAINTENANCE_POLL_MS = 60 * 60 * 1000;
const DRAFT_STORAGE_KEY = 'dust-wave-social-composer-draft';
const timezoneOptions = typeof Intl.supportedValuesOf === 'function'
    ? Intl.supportedValuesOf('timeZone')
    : ['UTC', 'America/Denver', 'America/Los_Angeles', 'America/New_York', 'Europe/London'];
let workerIntervalId = null;
let maintenanceIntervalId = null;
let notificationPermissionRequest = null;
const AudienceLineChart = defineAsyncComponent(() => import('@desktop/components/AudienceLineChart.vue'));
const BackupRestorePanel = defineAsyncComponent(() => import('@desktop/components/BackupRestorePanel.vue'));
const EmojiPickerPanel = defineAsyncComponent(() => import('@desktop/components/EmojiPickerPanel.vue'));
const PostDetailModal = defineAsyncComponent(() => import('@desktop/components/PostDetailModal.vue'));
const ProviderPreviewCard = defineAsyncComponent(() => import('@desktop/components/ProviderPreviewCard.vue'));
const SoftwareUpdatesPanel = defineAsyncComponent(() => import('@desktop/components/SoftwareUpdatesPanel.vue'));

function hasHtmlMarkup(value) {
    return /<\/?[a-z][\s\S]*>/i.test(String(value || ''));
}

function escapeHtml(value) {
    const element = document.createElement('div');
    element.textContent = String(value || '');

    return element.innerHTML;
}

function normalizeEditorContent(value) {
    const content = String(value || '');

    if (!content.trim()) {
        return '<div></div>';
    }

    if (hasHtmlMarkup(content)) {
        return content;
    }

    return content
        .split(/\r?\n/)
        .map((line) => `<div>${escapeHtml(line)}</div>`)
        .join('');
}

function editorBodyText(value) {
    const content = String(value || '');

    if (!content) {
        return '';
    }

    if (!hasHtmlMarkup(content)) {
        return content;
    }

    const element = document.createElement('div');
    element.innerHTML = content;
    const innerHTML = element.innerHTML;
    element.remove();

    let text = innerHTML
        .replace(/<div><\/div>/g, '\n')
        .replace(/<div>/g, '\n')
        .replace(/<\/div>/g, '')
        .replace(/<\/?[^>]+(>|$)/g, '');

    if (innerHTML.startsWith('<div>')) {
        text = text.substring(1);
    }

    return text.replace(/&nbsp;/g, ' ');
}

function compactEditorText(value) {
    return editorBodyText(value).split(/\s+/).filter(Boolean).join(' ');
}

const navigationViews = [
    {
        id: 'dashboard',
        label: 'Dashboard',
        section: 'Overview',
        description: 'Account analytics, publishing status, upcoming posts, and provider health.',
    },
    {
        id: 'posts',
        label: 'Posts',
        section: 'Publishing',
        description: 'Create, edit, validate, schedule, duplicate, retry, and delete social posts.',
    },
    {
        id: 'calendar',
        label: 'Calendar',
        section: 'Publishing',
        description: 'Review scheduled, published, failed, and draft posts by date, account, tag, and content.',
    },
    {
        id: 'media',
        label: 'Media',
        section: 'Library',
        description: 'Manage uploads, local imports, external downloads, stock images, GIFs, and media cleanup.',
    },
    {
        id: 'connections',
        label: 'Connections',
        section: 'Connections',
        description: 'Configure providers, connect social accounts, and manage imports from one place.',
    },
    {
        id: 'reports',
        label: 'Analytics',
        section: 'Analytics',
        description: 'Review audience and provider metrics for connected social accounts.',
    },
    {
        id: 'tags',
        label: 'Labels',
        section: 'Publishing',
        description: 'Create, edit, color-code, and delete labels used to organize posts.',
    },
    {
        id: 'settings',
        label: 'Settings',
        section: 'Workspace',
        description: 'Manage local identity, notifications, publishing defaults, and date and time display.',
    },
    {
        id: 'system',
        label: 'System',
        section: 'Operations',
        description: 'Monitor account health, queued work, provider limits, maintenance, and local system logs.',
    },
];
const navigationGroups = [
    {
        label: 'Work',
        viewIds: ['dashboard', 'posts', 'calendar', 'media'],
    },
    {
        label: 'Manage',
        viewIds: ['connections', 'reports', 'tags'],
    },
    {
        label: 'App',
        viewIds: ['settings', 'system'],
    },
].map((group) => ({
    ...group,
    views: group.viewIds.map((id) => navigationViews.find((view) => view.id === id)).filter(Boolean),
}));
const workspaceViewIds = ['connections', 'posts', 'media', 'tags'];
const activeView = ref('dashboard');
const activeConnectionTab = ref('accounts');
const activePostsMode = ref('compose');
const connectionWorkspaceTabs = [
    { id: 'accounts', label: 'Connected accounts' },
    { id: 'services', label: 'Provider setup' },
];
const providerReportDefinitions = {
    twitter: [
        { key: 'likes', label: 'Likes', description: 'The number of times where your posts were liked' },
        { key: 'retweets', label: 'Retweets', description: 'The number of times your tweets have been retweeted' },
        { key: 'impressions', label: 'Impressions', description: 'The number of times people saw your posts' },
    ],
    facebook_page: [
        { key: 'page_post_engagements', label: 'Post Engagements', description: 'The number of times people engaged with your posts through reactions, comments, shares, and more' },
        { key: 'page_posts_impressions', label: 'Posts Impressions', description: "The number of times your Page's posts entered a person screen" },
    ],
    instagram: [
        { key: 'likes', label: 'Likes', description: 'The number of likes on your Instagram media' },
        { key: 'comments', label: 'Comments', description: 'The number of comments on your Instagram media' },
        { key: 'media', label: 'Media', description: 'The number of imported Instagram media items' },
    ],
    mastodon: [
        { key: 'replies', label: 'Replies', description: 'The number of replies to your posts' },
        { key: 'reblogs', label: 'Reblogs', description: 'The number of times your posts have been reblogged' },
        { key: 'favourites', label: 'Favourites', description: 'The number of times your posts have been added to favourites' },
    ],
    tiktok: [
        { key: 'views', label: 'Views', description: 'The number of times your videos were viewed' },
        { key: 'likes', label: 'Likes', description: 'The number of likes on your videos' },
        { key: 'comments', label: 'Comments', description: 'The number of comments on your videos' },
        { key: 'shares', label: 'Shares', description: 'The number of times your videos were shared' },
    ],
};
const dustWaveTikTokBrokerUrl = 'https://dustwave-tiktok-broker.jogo.workers.dev';
const serviceDefinitions = [
    {
        id: 'facebook',
        label: 'Facebook',
        description: 'Store the Meta app credentials used for Facebook Pages, Instagram publishing, comments, and insights.',
        docsUrl: 'https://developers.facebook.com/docs/development/create-an-app/pages-use-case/',
        setupUrl: 'https://developers.facebook.com/apps',
        configurationSecretRef: 'secret://services/facebook',
        setupFields: [
            { key: 'redirect', label: 'OAuth callback URL', value: 'http://localhost/callback' },
            { key: 'scopes', label: 'Default OAuth scopes', value: 'business_management,pages_show_list,read_insights,pages_manage_posts,pages_read_engagement,pages_manage_engagement,instagram_basic,instagram_content_publish,instagram_manage_insights,instagram_manage_comments' },
        ],
        credentials: [
            { field: 'client_id', label: 'App ID', autocomplete: 'off' },
            { field: 'client_secret', label: 'App Secret', autocomplete: 'new-password', secret: true },
        ],
        configuration: [
            {
                field: 'api_version',
                label: 'API Version',
                defaultValue: 'v25.0',
                options: ['v25.0', 'v24.0', 'v23.0', 'v22.0', 'v21.0', 'v20.0', 'v19.0', 'v18.0', 'v17.0', 'v16.0'],
            },
        ],
    },
    {
        id: 'media_staging',
        label: 'Media Staging',
        description: 'Store the Cloudflare R2 Worker URL and token used to publish temporary local media URLs for providers such as Instagram.',
        docsUrl: 'https://developers.cloudflare.com/r2/api/workers/workers-api-usage/',
        setupUrl: 'https://dash.cloudflare.com/',
        configurationSecretRef: 'secret://services/media_staging',
        setupFields: [
            { key: 'worker', label: 'Worker', value: 'dustwave-media-staging' },
            { key: 'bucket', label: 'R2 bucket', value: 'dustwave-media-staging' },
            { key: 'retention', label: 'Retention', value: 'Temporary high-entropy URLs with 24 hour default TTL' },
        ],
        credentials: [
            { field: 'client_secret', label: 'Staging Token', autocomplete: 'new-password', secret: true },
        ],
        configuration: [
            {
                field: 'base_url',
                label: 'Base URL',
                defaultValue: 'https://dustwave-media-staging.jogo.workers.dev',
                input: 'text',
                placeholder: 'https://dustwave-media-staging.jogo.workers.dev',
            },
        ],
    },
    {
        id: 'twitter',
        label: 'X',
        description: 'Store the X developer app credentials used for OAuth, publishing, imports, and reports.',
        docsUrl: 'https://docs.x.com/fundamentals/developer-apps',
        setupUrl: 'https://developer.twitter.com/en/portal/projects-and-apps',
        configurationSecretRef: 'secret://services/twitter',
        setupFields: [
            { key: 'redirect', label: 'OAuth callback URL', value: 'http://localhost/callback' },
            { key: 'scopes', label: 'Default OAuth scopes', value: 'tweet.read tweet.write users.read offline.access' },
        ],
        credentials: [
            { field: 'client_id', label: 'API Key', autocomplete: 'off' },
            { field: 'client_secret', label: 'API Secret', autocomplete: 'new-password', secret: true },
        ],
        configuration: [
            {
                field: 'tier',
                label: 'Tier',
                defaultValue: 'pay_as_you_go',
                options: [
                    { value: 'pay_as_you_go', label: 'Pay as you go (Recommended)' },
                    { value: 'legacy', label: 'Legacy' },
                    { value: 'free', label: 'Free' },
                    { value: 'basic', label: 'Basic' },
                ],
            },
        ],
    },
    {
        id: 'tiktok',
        label: 'TikTok',
        description: 'Store the TikTok client key and broker settings used for assisted publishing, API-gated publishing modes, imports, and reports.',
        docsUrl: 'https://developers.tiktok.com/doc/overview/',
        setupUrl: 'https://developers.tiktok.com/',
        configurationSecretRef: 'secret://services/tiktok',
        setupFields: [
            { key: 'redirect', label: 'Broker OAuth callback URL', value: `${dustWaveTikTokBrokerUrl}/api/tiktok/oauth/callback` },
            { key: 'scopes', label: 'MVP analytics scopes', value: 'user.info.basic,user.info.stats,video.list' },
            { key: 'publishing_scopes', label: 'Future publishing scopes', value: 'video.upload,video.publish' },
            { key: 'secret', label: 'Client secret storage', value: 'Store TikTok client secret only in the Cloudflare broker, never in this desktop app.' },
        ],
        credentials: [
            { field: 'client_id', label: 'Client Key', autocomplete: 'off' },
        ],
        configuration: [
            {
                field: 'broker_base_url',
                label: 'Broker URL',
                defaultValue: dustWaveTikTokBrokerUrl,
                input: 'text',
                placeholder: dustWaveTikTokBrokerUrl,
            },
            {
                field: 'publishing_mode',
                label: 'Publishing Mode',
                defaultValue: 'assisted',
                options: [
                    { value: 'assisted', label: 'Assisted' },
                    { value: 'send_to_tiktok', label: 'Send to TikTok' },
                    { value: 'direct', label: 'Direct API' },
                ],
            },
        ],
    },
    {
        id: 'unsplash',
        label: 'Unsplash',
        description: 'Store the Unsplash API key used by the stock-photo search in the media library.',
        setupUrl: 'https://unsplash.com/oauth/applications',
        configurationSecretRef: 'secret://services/unsplash',
        setupFields: [
            { key: 'access', label: 'Required access', value: 'Public demo or production access key' },
        ],
        credentials: [
            { field: 'client_id', label: 'API Key', autocomplete: 'off' },
        ],
        configuration: [],
    },
    {
        id: 'klipy',
        label: 'Klipy',
        description: 'Store the Klipy API key used by GIF search.',
        docsUrl: 'https://klipy.com/developers',
        setupUrl: 'https://partner.klipy.com/',
        configurationSecretRef: 'secret://services/klipy',
        setupFields: [
            { key: 'access', label: 'Access path', value: 'Create a Partner Panel app, test with 100 calls/hour, then request production access.' },
            { key: 'attribution', label: 'Attribution', value: 'Follow Klipy branding and attribution guidance before public release.' },
        ],
        credentials: [
            { field: 'client_id', label: 'API Key', autocomplete: 'off' },
        ],
        configuration: [],
    },
];
const serviceConfigurationDefaults = serviceDefinitions.reduce((defaults, service) => {
    defaults[service.id] = service.configuration.reduce((configuration, field) => {
        configuration[field.field] = field.defaultValue;

        return configuration;
    }, {});

    return defaults;
}, {});
const serviceCredentialDraftDefaults = serviceDefinitions.reduce((drafts, service) => {
    drafts[service.id] = service.credentials.reduce((credentials, field) => {
        credentials[field.field] = '';

        return credentials;
    }, {});

    return drafts;
}, {});
const serviceActiveDraftDefaults = serviceDefinitions.reduce((drafts, service) => {
    drafts[service.id] = false;

    return drafts;
}, {});
const SOFTWARE_UPDATE_IDLE_STATUS = 'Not checked yet';
const dashboard = ref(null);
const credentialStatuses = ref([]);
const health = ref(null);
const settings = ref(null);
const settingsDraft = ref({
    timezone: 'UTC',
    date_format: 'human',
    time_format: 12,
    week_starts_on: 1,
    desktop_notifications: true,
    operator_name: '',
    admin_email: '',
    local_ai_media_labs: false,
    default_accounts: [],
});
const report = ref(null);
const reportAccountId = ref('');
const reportPeriod = ref('7_days');
const reportLoading = ref(false);
const reportError = ref('');
const activeReportAudienceIndex = ref(null);
const todayDate = () => {
    const date = new Date();
    date.setMinutes(date.getMinutes() - date.getTimezoneOffset());

    return date.toISOString().slice(0, 10);
};
const formatBytes = (value) => {
    const bytes = Number(value) || 0;

    if (bytes < 1024) {
        return `${bytes} B`;
    }

    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }

    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const pluralize = (count, singular, plural = `${singular}s`) => {
    const normalizedCount = Number(count) || 0;

    return `${normalizedCount} ${normalizedCount === 1 ? singular : plural}`;
};

const formatNumber = (value) => {
    if (value === null || value === undefined || Number.isNaN(Number(value))) {
        return '—';
    }

    return Number(value).toLocaleString();
};

const formatDelta = (value) => {
    if (value === null || value === undefined || Number.isNaN(Number(value))) {
        return '—';
    }

    const delta = Number(value);

    if (delta === 0) {
        return '0';
    }

    return `${delta > 0 ? '+' : ''}${delta.toLocaleString()}`;
};
const snapshot = ref({
    accounts: [],
    services: [],
    posts: [],
    media: [],
    tags: [],
    jobs: [],
    rate_limits: [],
    provider_capabilities: [],
});
const providerCapabilityMap = computed(() => {
    return new Map((snapshot.value.provider_capabilities || []).map((capability) => [capability.id, capability]));
});
const mediaLibrary = ref([]);
const mediaLibraryLoading = ref(false);
const mediaLibraryError = ref('');
const mediaTabs = [
    { id: 'uploads', label: 'Uploaded' },
    { id: 'stock', label: 'Stock' },
    { id: 'gifs', label: 'GIFs' },
];
const dustWaveMediaFileFilters = [
    {
        name: 'Dust Wave media',
        extensions: ['png', 'jpg', 'jpeg', 'gif', 'mp4', 'm4v'],
    },
];
const activeMediaTab = ref('uploads');
const selectedMediaIds = ref([]);
const selectedExternalMediaIds = ref([]);
const mediaFilter = ref({
    keyword: '',
    media_type: '',
});
const postQuery = ref({
    items: [],
    total: 0,
    page: 1,
    per_page: 25,
    total_pages: 1,
    has_failed_posts: false,
    calendar_window: null,
});
const postWorkspaceTabs = computed(() => [
    { id: 'compose', label: 'Compose' },
    { id: 'library', label: `Post library · ${postQuery.value.total}` },
]);
const postFilter = ref({
    status: '',
    keyword: '',
    calendar_type: 'month',
    date: todayDate(),
    accounts: [],
    tags: [],
    page: 1,
    per_page: 25,
});
const postFilterOpen = ref(false);
const postQueryLoading = ref(false);
const postQueryError = ref('');
const tagDraft = ref({
    name: '',
    hex_color: '#101215',
});
const editingTagUuid = ref('');
const tagEditDraft = ref({
    name: '',
    hex_color: '#101215',
});
const activeServiceTab = ref('facebook');
const serviceCredentialDrafts = ref(JSON.parse(JSON.stringify(serviceCredentialDraftDefaults)));
const serviceConfigurationDrafts = ref(JSON.parse(JSON.stringify(serviceConfigurationDefaults)));
const serviceActiveDrafts = ref({ ...serviceActiveDraftDefaults });
const twitterOAuthDraft = ref({
    redirect_uri: 'http://localhost/callback',
    code: '',
    code_verifier: '',
});
const facebookOAuthDraft = ref({
    redirect_uri: 'http://localhost/callback',
    code: '',
    selected_pages: [],
    selected_instagram_accounts: [],
});
const mastodonAppDraft = ref({
    server: '',
    client_name: 'Dust Wave Social',
    redirect_uri: 'urn:ietf:wg:oauth:2.0:oob',
    website: '',
});
const mastodonOAuthDraft = ref({
    server: '',
    code: '',
    redirect_uri: 'urn:ietf:wg:oauth:2.0:oob',
});
const tiktokConnectionDraft = ref({
    provider_id: '',
    name: '',
    username: '',
    avatar_path: '',
    broker_connection: '',
    scopes: 'user.info.basic,user.info.stats,video.list',
});
const mediaImport = ref({
    source_path: '',
    name: '',
});
const mediaDownload = ref({
    url: '',
    name: '',
    source: 'url',
});
const externalMediaSearch = ref({
    source: 'stock',
    keyword: '',
    page: 1,
});
const externalMediaResults = ref(null);
const externalMediaLoading = ref(false);
const externalMediaError = ref('');
const tagSaving = ref(false);
const tagError = ref('');
const serviceSaving = ref(false);
const serviceError = ref('');
const serviceSaveMessage = ref('');
const serviceSetupCopied = ref('');
const twitterOAuthSaving = ref(false);
const twitterOAuthError = ref('');
const twitterOAuthStart = ref(null);
const twitterOAuthConnection = ref(null);
const facebookOAuthSaving = ref(false);
const facebookOAuthError = ref('');
const facebookOAuthStart = ref(null);
const facebookUserConnection = ref(null);
const facebookPageConnection = ref(null);
const instagramAccountConnection = ref(null);
const mastodonAppSaving = ref(false);
const mastodonAppError = ref('');
const mastodonAppRegistration = ref(null);
const mastodonOAuthSaving = ref(false);
const mastodonOAuthError = ref('');
const mastodonOAuthConnection = ref(null);
const tiktokConnectionSaving = ref(false);
const tiktokConnectionError = ref('');
const tiktokConnection = ref(null);
const addAccountModalOpen = ref(false);
const activeAccountProvider = ref('');
const accountProviderChoices = [
    {
        id: 'twitter',
        label: 'X / Twitter',
        description: 'Authorize an X account with the configured X developer app.',
        service: 'twitter',
    },
    {
        id: 'facebook',
        label: 'Facebook / Instagram',
        description: 'Authorize Meta, then choose Facebook Pages and Instagram accounts.',
        service: 'facebook',
    },
    {
        id: 'mastodon',
        label: 'Mastodon',
        description: 'Register a server app, authorize it, and connect the returned account.',
        service: '',
    },
    {
        id: 'tiktok',
        label: 'TikTok',
        description: 'Use the broker-assisted account connection and analytics workflow.',
        service: 'tiktok',
    },
];
const accountSaving = ref(false);
const accountError = ref('');
const accountOnboardingCopied = ref('');
const accountRefreshingUuid = ref('');
const accountImportingUuid = ref('');
const accountQueuingUuid = ref('');
const queueAllImportsRunning = ref(false);
const mastodonImportSummary = ref(null);
const twitterImportSummary = ref(null);
const facebookImportSummary = ref(null);
const instagramImportSummary = ref(null);
const tiktokImportSummary = ref(null);
const queuedImportJob = ref(null);
const queuedImportBatch = ref(null);
const queuedImportError = ref('');
const mediaSaving = ref(false);
const mediaError = ref('');
const mediaProgress = ref('');
const mediaImportResults = ref([]);
const mediaCleanup = ref(null);
const mediaDropActive = ref(false);
const localAiRuntime = ref({
    status: 'not_checked',
    accelerator: 'unknown',
    webgpu: false,
    wasm: false,
    models: 0,
    compiled_model: '',
    detail: 'Not checked',
});
const localAiBusy = ref(false);
const localAiError = ref('');
const localAiResult = ref(null);
const localAiSearchQuery = ref('');
const localAiSearchResults = ref(null);
const localAiCropRatio = ref('square');
let localAiLiteRtLoaded = false;
let localAiAbortController = null;
const LOCAL_AI_UPSCALE_TILE_SIZE = 128;
const LOCAL_AI_UPSCALE_SCALE_FACTOR = 4;
const LOCAL_AI_UPSCALE_MAX_SOURCE_EDGE = 512;
const draftBody = ref('');
const draftAccountIds = ref([]);
const draftAccountBodies = ref({});
const activeDraftVersion = ref(0);
const versionPickerOpen = ref(false);
const emojiPickerOpen = ref(false);
const draftMediaIds = ref([]);
const draftExternalMedia = ref([]);
const draftTagIds = ref([]);
const draftScheduledAt = ref('');
const editingPostUuid = ref('');
const contextualEditOrigin = ref('');
const draftSaving = ref(false);
const draftError = ref('');
const tagPickerOpen = ref(false);
const tagSearchText = ref('');
const validationRunning = ref(false);
const validationError = ref('');
const validationReport = ref(null);
const postNowConfirmationOpen = ref(false);
const postScheduleDrafts = ref({});
const editingSchedulePostUuid = ref('');
const scheduleSaving = ref(false);
const scheduleError = ref('');
const selectedPostUuids = ref([]);
const bulkDeleteSummary = ref(null);
const selectedPostSummary = ref(null);
const selectedPostDetail = ref(null);
const postDetailLoading = ref(false);
const postDetailError = ref('');
const workerRunning = ref(false);
const systemLogs = ref([]);
const systemLogRunning = ref(false);
const systemLogError = ref('');
const systemLogExport = ref(null);
const systemLogClearSummary = ref(null);
const backupRunning = ref(false);
const backupError = ref('');
const backupSummary = ref(null);
const restoreRunning = ref(false);
const restoreError = ref('');
const restoreSummary = ref(null);
const restoreBackupPath = ref('');
const desktopNotificationError = ref('');
const desktopNotificationTestSent = ref(false);
const healthNotificationKey = ref('');
const appDataPathCopied = ref(false);
const systemStatusCopied = ref(false);
const systemStatusCopyError = ref('');
const softwareUpdateChecking = ref(false);
const softwareUpdateInstalling = ref(false);
const softwareUpdateStatus = ref(SOFTWARE_UPDATE_IDLE_STATUS);
const softwareUpdateError = ref('');
const softwareUpdateProgress = ref('');
// Tauri updater resources carry private class state and must not be wrapped in a Vue proxy.
const softwareUpdateAvailable = shallowRef(null);
const settingsSaving = ref(false);
const settingsError = ref('');
const settingsSaved = ref(false);
const maintenanceRunning = ref(false);
const maintenanceError = ref('');
const maintenanceSummary = ref(null);
const desktopMaintenanceRunning = ref(false);
const desktopMaintenanceError = ref('');
const desktopMaintenanceSummary = ref(null);
const autoMaintenanceLastRun = ref('');
const staleRecoveryRunning = ref(false);
const staleRecoveryError = ref('');
const staleRecoverySummary = ref(null);
const failedImportRetryRunning = ref(false);
const failedImportRetryError = ref('');
const failedImportRetryJobs = ref([]);
const loadError = ref('');
const confirmationDialog = ref({
    open: false,
    title: '',
    description: '',
    confirmLabel: 'Continue',
    danger: false,
});
let confirmationResolver = null;

const requestConfirmation = ({
    title,
    description,
    confirmLabel = 'Continue',
    danger = false,
}) => new Promise((resolve) => {
    if (confirmationResolver) {
        confirmationResolver(false);
    }

    confirmationResolver = resolve;
    confirmationDialog.value = {
        open: true,
        title,
        description,
        confirmLabel,
        danger,
    };
});

const resolveConfirmation = (accepted) => {
    const resolve = confirmationResolver;
    confirmationResolver = null;
    confirmationDialog.value = {
        ...confirmationDialog.value,
        open: false,
    };
    resolve?.(accepted);
};

const writeClipboard = async (text, errorTarget) => {
    try {
        await navigator.clipboard.writeText(text);
        return true;
    } catch (error) {
        if (errorTarget) {
            errorTarget.value = String(error);
        }

        return false;
    }
};

const openAddAccountModal = (provider = '') => {
    activeAccountProvider.value = provider;
    accountError.value = '';
    addAccountModalOpen.value = true;
};

const closeAddAccountModal = () => {
    addAccountModalOpen.value = false;
    activeAccountProvider.value = '';
};

const postFilterTotal = computed(() => {
    return postFilter.value.accounts.length + postFilter.value.tags.length;
});

const activeViewDefinition = computed(() => {
    return navigationViews.find((view) => view.id === activeView.value) || navigationViews[0];
});

const contextualEditOriginLabel = computed(() => {
    return navigationViews.find((view) => view.id === contextualEditOrigin.value)?.label || 'previous view';
});

const isWorkspaceView = computed(() => {
    return workspaceViewIds.includes(activeView.value);
});

const attentionNotices = computed(() => {
    const counts = health.value?.counts;

    if (!counts) {
        return [];
    }

    const notices = [];

    if (counts.unauthorized_accounts > 0) {
        notices.push({
            severity: 'error',
            title: 'Account connection lost',
            detail: `${counts.unauthorized_accounts} account(s) need to be refreshed or reconnected.`,
            view: 'connections',
            connectionTab: 'accounts',
        });
    }

    if (counts.failed_posts > 0) {
        notices.push({
            severity: 'error',
            title: 'Failed posts',
            detail: `${counts.failed_posts} post(s) need review before they can publish.`,
            view: 'posts',
            status: 'failed',
        });
    }

    if (counts.failed_jobs > 0) {
        notices.push({
            severity: 'warning',
            title: 'Failed background work',
            detail: `${counts.failed_jobs} background item(s) need recovery or review.`,
            view: 'system',
        });
    }

    if (counts.rate_limits > 0) {
        notices.push({
            severity: 'warning',
            title: 'Provider limits active',
            detail: `${counts.rate_limits} provider limit(s) are delaying work.`,
            view: 'system',
        });
    }

    const representedIssueTitles = new Set([
        'Unauthorized accounts',
        'Failed posts',
        'Failed background work',
        'Active rate limits',
    ]);

    for (const issue of health.value?.issues || []) {
        if (representedIssueTitles.has(issue.title)) {
            continue;
        }

        notices.push({
            severity: issue.severity === 'error' ? 'error' : 'warning',
            title: issue.title,
            detail: issue.detail,
            view: issue.title === 'Missing active service credentials' ? 'connections' : 'system',
            connectionTab: issue.title === 'Missing active service credentials' ? 'services' : undefined,
        });
    }

    return notices;
});

const reportMetricMap = computed(() => {
    return new Map((report.value?.metrics || []).map((metric) => [metric.key, metric.value]));
});

const providerReportCards = computed(() => {
    const definitions = providerReportDefinitions[report.value?.provider] || [];

    return definitions.map((definition) => ({
        ...definition,
        value: reportMetricMap.value.get(definition.key) || 0,
    }));
});

const activeReportAccount = computed(() => {
    return snapshot.value.accounts.find((account) => Number(account.id) === Number(reportAccountId.value)) || null;
});

const reportAudiencePoints = computed(() => {
    const points = report.value?.audience?.points || [];

    return points.map((point, index) => {
        const value = point.value === null || point.value === undefined ? null : Number(point.value) || 0;
        const previousPoint = points[index - 1];
        const previousValue = previousPoint?.value === null || previousPoint?.value === undefined
            ? null
            : Number(previousPoint?.value) || 0;

        return {
            ...point,
            value,
            index,
            delta: previousValue === null || value === null ? null : value - previousValue,
        };
    });
});

const activeReportAudiencePoint = computed(() => {
    if (!reportAudiencePoints.value.length) {
        return null;
    }

    const index = Number(activeReportAudienceIndex.value);

    return reportAudiencePoints.value[index] || reportAudiencePoints.value[reportAudiencePoints.value.length - 1];
});

const reportAudienceSummary = computed(() => {
    const values = reportAudiencePoints.value
        .map((point) => point.value)
        .filter((value) => value !== null && value !== undefined);

    if (!values.length) {
        return null;
    }

    const first = values[0];
    const last = values[values.length - 1];
    const average = Math.round(values.reduce((total, value) => total + value, 0) / values.length);
    const high = Math.max(...values);

    return {
        change: last - first,
        average,
        high,
    };
});

const configuredCredentialCount = computed(() => {
    return credentialStatuses.value.filter((status) => status.configured).length;
});

const activeCredentialCount = computed(() => {
    return serviceDefinitions.filter((service) => serviceReady(service.id)).length;
});

const serviceReadinessSummary = computed(() => {
    return `${activeCredentialCount.value}/${serviceDefinitions.length} active`;
});

const activeServiceDefinition = computed(() => {
    return serviceDefinitions.find((service) => service.id === activeServiceTab.value) || serviceDefinitions[0];
});

const activeServiceStatus = computed(() => {
    return credentialStatuses.value.find((status) => status.service === activeServiceTab.value) || null;
});

const activeServiceRecord = computed(() => {
    return snapshot.value.services.find((service) => service.name === activeServiceTab.value) || null;
});

const activeServiceConfiguredFields = computed(() => {
    return new Map((activeServiceStatus.value?.fields || []).map((field) => [field.field, field]));
});

const activeServiceIsReady = computed(() => {
    return serviceReady(activeServiceTab.value);
});

const systemLogEntryCount = computed(() => {
    return systemLogs.value.reduce((total, log) => total + (Number(log.entry_count) || 0), 0);
});

const systemLogBytes = computed(() => {
    return systemLogs.value.reduce((total, log) => total + (Number(log.bytes) || 0), 0);
});

const systemTechnicalRows = computed(() => [
    {
        label: 'Runtime',
        value: 'Tauri desktop',
    },
    {
        label: 'FFmpeg',
        value: health.value?.media_tools?.ffmpeg
            ? `${health.value.media_tools.ffmpeg.available ? 'Installed' : 'Not installed'} (${health.value.media_tools.ffmpeg.command})`
            : 'Unknown',
    },
    {
        label: 'FFprobe',
        value: health.value?.media_tools?.ffprobe
            ? `${health.value.media_tools.ffprobe.available ? 'Installed' : 'Not installed'} (${health.value.media_tools.ffprobe.command})`
            : 'Unknown',
    },
    {
        label: 'Workspace',
        value: 'Local',
    },
    {
        label: 'Services active',
        value: `${activeCredentialCount.value}/${serviceDefinitions.length}`,
    },
    {
        label: 'Accounts',
        value: `${dashboard.value?.accounts?.authorized ?? 0}/${dashboard.value?.accounts?.total ?? 0} connected`,
    },
    {
        label: 'Media library',
        value: `${mediaLibrary.value.length} items`,
    },
    {
        label: 'System logs',
        value: `${systemLogs.value.length} files · ${systemLogEntryCount.value} entries · ${formatBytes(systemLogBytes.value)}`,
    },
    {
        label: 'Auto maintenance',
        value: autoMaintenanceLastRun.value || 'Not run yet',
    },
]);

const softwareUpdateBadge = computed(() => {
    if (softwareUpdateInstalling.value) {
        return 'Installing';
    }

    if (softwareUpdateChecking.value) {
        return 'Checking';
    }

    if (softwareUpdateAvailable.value) {
        return 'Available';
    }

    if (softwareUpdateError.value) {
        return 'Attention';
    }

    return 'Ready';
});

const postGroups = computed(() => {
    const groups = new Map();

    for (const post of postQuery.value.items || []) {
        const key = postDateKey(post);

        if (!groups.has(key)) {
            groups.set(key, []);
        }

        groups.get(key).push(post);
    }

    return Array.from(groups.entries()).map(([date, posts]) => ({ date, posts }));
});

const postDateKey = (post) => {
    return post.scheduled_at?.slice(0, 10) || post.published_at?.slice(0, 10) || post.updated_at?.slice(0, 10) || '';
};

const postHour = (post) => {
    const time = postTime(post);

    if (!time) {
        return null;
    }

    const hour = Number(time.slice(0, 2));

    return Number.isFinite(hour) ? hour : null;
};

const parseLocalDate = (value) => {
    const [year, month, day] = value.split('-').map((part) => Number(part));

    return new Date(year, month - 1, day);
};

const formatLocalDate = (date) => {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');

    return `${year}-${month}-${day}`;
};

const addDays = (date, days) => {
    const next = new Date(date);
    next.setDate(next.getDate() + days);

    return next;
};

const calendarCells = computed(() => {
    const selected = parseLocalDate(postFilter.value.date);
    const weekStartsOn = Number(settings.value?.week_starts_on ?? 1);
    const postsByDate = new Map();

    for (const post of postQuery.value.items || []) {
        const key = postDateKey(post);

        if (!postsByDate.has(key)) {
            postsByDate.set(key, []);
        }

        postsByDate.get(key).push(post);
    }

    let start = selected;
    let days = 1;

    if (postFilter.value.calendar_type === 'week') {
        const day = weekStartsOn === 0 ? selected.getDay() : (selected.getDay() + 6) % 7;
        start = addDays(selected, -day);
        days = 7;
    }

    if (postFilter.value.calendar_type === 'month') {
        const first = new Date(selected.getFullYear(), selected.getMonth(), 1);
        const day = weekStartsOn === 0 ? first.getDay() : (first.getDay() + 6) % 7;
        start = addDays(first, -day);
        days = 42;
    }

    return Array.from({ length: days }, (_, index) => {
        const date = addDays(start, index);
        const key = formatLocalDate(date);

        return {
            date: key,
            day_number: date.getDate(),
            weekday: date.toLocaleDateString(undefined, { weekday: 'short' }),
            is_selected: key === postFilter.value.date,
            is_current_month: date.getMonth() === selected.getMonth(),
            posts: postsByDate.get(key) || [],
        };
    });
});

const calendarWeekSlots = computed(() => {
    const format = Number(settings.value?.time_format ?? 12);

    return Array.from({ length: 24 }, (_, hour) => {
        const value = `${String(hour).padStart(2, '0')}:00`;
        const label = format === 24
            ? value
            : `${hour % 12 || 12}:00 ${hour < 12 ? 'AM' : 'PM'}`;

        return {
            hour,
            value,
            label,
        };
    });
});

const calendarSlotPosts = (date, hour) => {
    return (postQuery.value.items || []).filter((post) => {
        return postDateKey(post) === date && postHour(post) === hour;
    });
};

const calendarTitle = computed(() => {
    const selected = parseLocalDate(postFilter.value.date);

    if (Number.isNaN(selected.getTime())) {
        return postFilter.value.date;
    }

    if (postFilter.value.calendar_type === 'day') {
        return selected.toLocaleDateString(undefined, {
            weekday: 'long',
            month: 'short',
            day: 'numeric',
            year: 'numeric',
        });
    }

    if (postFilter.value.calendar_type === 'week') {
        const weekStartsOn = Number(settings.value?.week_starts_on ?? 1);
        const day = weekStartsOn === 0 ? selected.getDay() : (selected.getDay() + 6) % 7;
        const start = addDays(selected, -day);
        const end = addDays(start, 6);

        return `${start.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} - ${end.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}`;
    }

    return selected.toLocaleDateString(undefined, {
        month: 'long',
        year: 'numeric',
    });
});

const calendarWeekdayLabels = computed(() => {
    const weekStartsOn = Number(settings.value?.week_starts_on ?? 1);
    const labels = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

    return Array.from({ length: 7 }, (_, index) => labels[(weekStartsOn + index) % 7]);
});

const moveCalendar = async (direction) => {
    const selected = parseLocalDate(postFilter.value.date);
    const next = new Date(selected);

    if (postFilter.value.calendar_type === 'month') {
        next.setMonth(next.getMonth() + direction);
    } else if (postFilter.value.calendar_type === 'week') {
        next.setDate(next.getDate() + direction * 7);
    } else {
        next.setDate(next.getDate() + direction);
    }

    await selectCalendarDate(formatLocalDate(next));
};

const selectToday = async () => {
    await selectCalendarDate(todayDate());
};

const mediaLibraryById = computed(() => {
    return new Map(mediaLibrary.value.map((item) => [item.id, item]));
});

const selectedDraftAccounts = computed(() => {
    const selected = new Set(draftAccountIds.value.map((id) => Number(id)));

    return snapshot.value.accounts.filter((account) => selected.has(Number(account.id)));
});

const selectedDraftProviderCounts = computed(() => {
    return selectedDraftAccounts.value.reduce((counts, account) => {
        const key = providerKey(account.provider);
        counts[key] = (counts[key] || 0) + 1;

        return counts;
    }, {});
});

const draftProviderWarnings = computed(() => {
    return Object.entries(selectedDraftProviderCounts.value)
        .filter(([provider, count]) => {
            return count > 1 && providerPostRule(provider)?.simultaneous_posting === false;
        })
        .map(([provider]) => {
            return `${providerDisplayName(provider)} does not allow simultaneous posting to multiple accounts. Create a separate post for the extra account.`;
        });
});

const draftVersionAccountIds = computed(() => {
    return new Set(
        Object.keys(draftAccountBodies.value)
            .map((accountId) => Number(accountId)),
    );
});

const availableDraftVersionAccounts = computed(() => {
    const versioned = draftVersionAccountIds.value;

    return selectedDraftAccounts.value.filter((account) => !versioned.has(Number(account.id)));
});

const draftVersionCount = computed(() => {
    return draftVersionTabs.value.length;
});

const draftVersionTabs = computed(() => {
    const original = {
        id: 0,
        key: 'original',
        label: 'Original',
        sublabel: 'Base copy',
        provider: 'original',
        account: null,
    };

    return [
        original,
        ...selectedDraftAccounts.value
            .filter((account) => draftVersionAccountIds.value.has(Number(account.id)))
            .map((account) => ({
                id: Number(account.id),
                key: `account-${account.id}`,
                label: account.name || account.username || providerDisplayName(account.provider),
                sublabel: providerDisplayName(account.provider),
                provider: account.provider,
                account,
            })),
    ];
});

const activeDraftVersionTab = computed(() => {
    return draftVersionTabs.value.find((tab) => Number(tab.id) === Number(activeDraftVersion.value)) || draftVersionTabs.value[0];
});

const activeDraftBody = computed({
    get() {
        const accountId = Number(activeDraftVersion.value);

        if (!accountId) {
            return draftBody.value;
        }

        return draftAccountBodies.value[accountId] || '';
    },
    set(value) {
        const accountId = Number(activeDraftVersion.value);

        if (!accountId) {
            draftBody.value = value;
            return;
        }

        draftAccountBodies.value = {
            ...draftAccountBodies.value,
            [accountId]: value,
        };
    },
});

const activeDraftCharacterLabel = computed(() => {
    const body = editorBodyText(activeDraftBody.value);
    const limit = providerCharacterLimit(activeDraftVersionTab.value?.provider);

    return limit ? `${body.length}/${limit}` : `${body.length} chars`;
});

const activeDraftOverLimit = computed(() => {
    const body = editorBodyText(activeDraftBody.value);
    const limit = providerCharacterLimit(activeDraftVersionTab.value?.provider);

    return Boolean(limit && body.length > limit);
});

const draftPreviewMedia = computed(() => {
    const localMedia = draftMediaIds.value
        .map((id) => mediaResource({ id: Number(id) }))
        .filter(Boolean);

    return [
        ...localMedia,
        ...externalMediaPreviewResources(draftExternalMedia.value),
    ];
});

const draftMediaKindCounts = computed(() => {
    return draftPreviewMedia.value.reduce((counts, item) => {
        const mediaType = String(item.media_type || '').toLowerCase();
        const mimeType = String(item.mime_type || '').toLowerCase();

        if (mediaType === 'gif' || mimeType.includes('gif')) {
            counts.gifs += 1;
        } else if (mediaType === 'video' || mimeType.startsWith('video/')) {
            counts.videos += 1;
        } else if (mediaType === 'image' || mimeType.startsWith('image/')) {
            counts.photos += 1;
        }

        return counts;
    }, {
        photos: 0,
        videos: 0,
        gifs: 0,
    });
});

const draftMediaMixed = computed(() => {
    return Object.values(draftMediaKindCounts.value).filter((count) => count > 0).length > 1;
});

const draftValidationMessages = computed(() => {
    const messages = [];

    messages.push(...draftProviderWarnings.value);

    selectedDraftAccounts.value.forEach((account) => {
        const provider = providerKey(account.provider);
        const capability = providerCapability(provider);
        const rule = capability?.post_config;

        if (!rule) {
            return;
        }

        if (capability.supports_publish === false) {
            messages.push(`${providerDisplayName(account.provider)} uses assisted publishing or requires approved direct API publishing before scheduling.`);
        }

        const text = editorBodyText(draftAccountBodies.value[account.id] ?? draftBody.value).trim();

        if (text.length > rule.max_text_chars) {
            messages.push(`${account.name || account.username || providerDisplayName(account.provider)} is ${text.length - rule.max_text_chars} character(s) over the ${providerDisplayName(account.provider)} limit.`);
        }

        for (const [kind, count] of Object.entries(draftMediaKindCounts.value)) {
            const limit = rule.max_media[kind];

            if (count > limit) {
                messages.push(`${providerDisplayName(account.provider)} allows ${limit} ${kind}; this draft has ${count}.`);
            }
        }

        if (!rule.max_media.allow_mixing && draftMediaMixed.value) {
            messages.push(`${providerDisplayName(account.provider)} does not allow mixed media types in one post.`);
        }
    });

    return Array.from(new Set(messages));
});

const draftPreviewCards = computed(() => {
    const accounts = selectedDraftAccounts.value.length
        ? selectedDraftAccounts.value
        : [{ id: 0, uuid: 'original', provider: 'Original', name: 'All accounts', username: '' }];

    return accounts.map((account) => buildProviderPreviewCard({
        account,
        body: editorBodyText(draftAccountBodies.value[account.id] || draftBody.value || '').trim(),
        media: draftPreviewMedia.value,
    }));
});

const selectedPostAccounts = computed(() => {
    const detail = selectedPostDetail.value;
    const summaryAccounts = selectedPostSummary.value?.accounts || [];
    const summaryById = new Map(summaryAccounts.map((account) => [Number(account.id), account]));
    const snapshotById = new Map(snapshot.value.accounts.map((account) => [Number(account.id), account]));
    const ids = detail?.accounts?.length
        ? detail.accounts.map((id) => Number(id))
        : summaryAccounts.map((account) => Number(account.id));

    return ids
        .map((id) => snapshotById.get(id) || summaryById.get(id))
        .filter(Boolean);
});

const selectedPostTags = computed(() => {
    const detailTagIds = new Set((selectedPostDetail.value?.tags || []).map((id) => Number(id)));

    if (!detailTagIds.size) {
        return selectedPostSummary.value?.tags || [];
    }

    const summaryById = new Map((selectedPostSummary.value?.tags || []).map((tag) => [Number(tag.id), tag]));

    return Array.from(detailTagIds)
        .map((id) => snapshot.value.tags.find((tag) => Number(tag.id) === id) || summaryById.get(id))
        .filter(Boolean);
});

const selectedPostOriginalVersion = computed(() => {
    const versions = selectedPostDetail.value?.versions || [];

    return versions.find((version) => version.is_original) || versions[0] || null;
});

const selectedPostVersionForAccount = (accountId) => {
    const versions = selectedPostDetail.value?.versions || [];

    return versions.find((version) => Number(version.account_id) === Number(accountId)) || selectedPostOriginalVersion.value;
};

const selectedPostContentMedia = (content) => {
    const mediaIds = Array.isArray(content?.media) ? content.media : [];
    const localMedia = mediaIds
        .map((id) => mediaResource({ id: Number(id) }))
        .filter(Boolean);
    const externalMedia = externalMediaPreviewResources(content?.external_media || []);
    const media = [...localMedia, ...externalMedia];

    if (media.length) {
        return media;
    }

    return [
        ...(selectedPostSummary.value?.media || []),
        ...externalMediaPreviewResources(selectedPostSummary.value?.external_media || []),
    ];
};

const postSummaryMedia = (post) => [
    ...(post?.media || []),
    ...externalMediaPreviewResources(post?.external_media || []),
];

const selectedPostPreviewCards = computed(() => {
    if (!selectedPostDetail.value) {
        return [];
    }

    const accounts = selectedPostAccounts.value.length
        ? selectedPostAccounts.value
        : [{ id: 0, uuid: 'original', provider: 'Original', name: 'All accounts', username: '' }];

    return accounts.map((account) => {
        const version = selectedPostVersionForAccount(account.id);
        const content = version?.content?.[0] || { body: '', media: [] };

        return buildProviderPreviewCard({
            account,
            body: editorBodyText(content.body || '').trim(),
            media: selectedPostContentMedia(content),
            hasCustomVersion: Boolean(version && !version.is_original),
        });
    });
});

const selectedPostSummaryMedia = computed(() => postSummaryMedia(selectedPostSummary.value));

const selectedPostTimeline = computed(() => {
    const summary = selectedPostSummary.value;
    const detail = selectedPostDetail.value;

    if (!summary && !detail) {
        return [];
    }

    const items = [];

    if (summary?.created_at) {
        items.push({
            label: 'Created',
            detail: 'Draft was created',
            at: summary.created_at,
            tone: 'neutral',
        });
    }

    if (detail?.scheduled_at || summary?.scheduled_at) {
        items.push({
            label: 'Scheduled',
            detail: `Schedule status: ${detail?.schedule_status || summary?.schedule_status || 'scheduled'}`,
            at: detail?.scheduled_at || summary?.scheduled_at,
            tone: 'scheduled',
        });
    }

    if (detail?.published_at || summary?.published_at) {
        items.push({
            label: 'Published',
            detail: 'Provider publishing completed',
            at: detail?.published_at || summary?.published_at,
            tone: 'published',
        });
    }

    for (const message of summary?.failure_errors || []) {
        items.push({
            label: 'Failed',
            detail: message,
            at: summary.updated_at || detail?.scheduled_at || '',
            tone: 'failed',
        });
    }

    if (summary?.updated_at && summary.updated_at !== summary.created_at) {
        items.push({
            label: 'Updated',
            detail: `Current status: ${detail?.status || summary.status}`,
            at: summary.updated_at,
            tone: 'neutral',
        });
    }

    return items;
});

const visiblePostUuids = computed(() => {
    return (postQuery.value.items || []).map((post) => post.uuid);
});

const allVisiblePostsSelected = computed(() => {
    return visiblePostUuids.value.length > 0
        && visiblePostUuids.value.every((uuid) => selectedPostUuids.value.includes(uuid));
});

const postPageStart = computed(() => {
    if (!postQuery.value.total) {
        return 0;
    }

    return ((postQuery.value.page || 1) - 1) * (postQuery.value.per_page || postFilter.value.per_page) + 1;
});

const postPageEnd = computed(() => {
    if (!postQuery.value.total || !postQuery.value.items?.length) {
        return 0;
    }

    return Math.min(postQuery.value.total, postPageStart.value + (postQuery.value.items?.length || 0) - 1);
});

const selectedExternalMediaItems = computed(() => {
    const selected = new Set(selectedExternalMediaIds.value);

    return (externalMediaResults.value?.items || []).filter((item) => selected.has(item.id));
});

const selectedMediaCount = computed(() => {
    return activeMediaTab.value === 'uploads'
        ? selectedMediaIds.value.length
        : selectedExternalMediaItems.value.length;
});

const selectedDraftTags = computed(() => {
    const selected = new Set(draftTagIds.value.map((id) => Number(id)));

    return snapshot.value.tags.filter((tag) => selected.has(Number(tag.id)));
});

const availableDraftTags = computed(() => {
    const selected = new Set(draftTagIds.value.map((id) => Number(id)));
    const search = tagSearchText.value.trim().toLowerCase();

    return snapshot.value.tags.filter((tag) => {
        const matchesSearch = !search || tag.name.toLowerCase().includes(search);

        return matchesSearch && !selected.has(Number(tag.id));
    });
});

const importableProviders = computed(() => {
    return new Set((snapshot.value.provider_capabilities || [])
        .filter((capability) => capability.supports_audience_import || capability.supports_metrics_import)
        .map((capability) => capability.id));
});
const accountImportSupported = (account) => {
    return importableProviders.value.has(providerKey(account.provider));
};
const connectedImportAccountCount = computed(() => {
    return snapshot.value.accounts.filter((account) => account.authorized && accountImportSupported(account)).length;
});

const navigationBadge = (id) => {
    if (id === 'dashboard') {
        return `${dashboard.value?.posts?.scheduled ?? 0} scheduled`;
    }

    if (id === 'posts') {
        return `${postQuery.value.total || snapshot.value.posts.length} posts`;
    }

    if (id === 'calendar') {
        return postWindowLabel.value || 'Month view';
    }

    if (id === 'media') {
        return `${mediaLibrary.value.length} media`;
    }

    if (id === 'connections') {
        return `${dashboard.value?.accounts?.authorized ?? 0} accounts · ${activeCredentialCount.value}/${serviceDefinitions.length} services`;
    }

    if (id === 'reports') {
        return reportPeriod.value.replaceAll('_', ' ');
    }

    if (id === 'tags') {
        return `${snapshot.value.tags.length} tags`;
    }

    if (id === 'settings') {
        return settings.value?.timezone || 'Timezone';
    }

    if (id === 'system') {
        return health.value?.status?.replaceAll('_', ' ') || 'Health';
    }

    return '';
};

const mediaResource = (item) => {
    return mediaLibraryById.value.get(item.id) || null;
};

const mediaAssetUrl = (path) => {
    if (!path) {
        return '';
    }

    if (path.startsWith('http://') || path.startsWith('https://')) {
        return path;
    }

    return convertFileSrc(path);
};

const mediaThumbnailUrl = (item) => {
    return item?.thumb_url || item?.url || '';
};

const localAiLabsEnabled = computed(() => {
    return Boolean(settings.value?.local_ai_media_labs || settingsDraft.value.local_ai_media_labs);
});

const localAiStaticImage = (item) => {
    return String(item?.mime_type || '').startsWith('image/') && item?.mime_type !== 'image/gif';
};

const fileNameFromPath = (path) => {
    return String(path || '').split(/[\\/]/).filter(Boolean).pop() || String(path || 'Media file');
};

const providerKey = (provider) => {
    const key = String(provider || '').toLowerCase();

    if (key.includes('instagram')) {
        return 'instagram';
    }

    if (key.includes('facebook')) {
        return 'facebook_page';
    }

    if (key.includes('mastodon')) {
        return 'mastodon';
    }

    if (key.includes('tiktok')) {
        return 'tiktok';
    }

    if (key.includes('twitter') || key === 'x') {
        return 'twitter';
    }

    return 'original';
};

const providerCapability = (provider) => {
    return providerCapabilityMap.value.get(providerKey(provider)) || null;
};

const providerPostRule = (provider) => {
    return providerCapability(provider)?.post_config || null;
};

const providerDisplayName = (provider) => {
    const key = providerKey(provider);
    const capability = providerCapabilityMap.value.get(key);

    if (capability?.display_name) {
        return capability.display_name;
    }

    if (key === 'facebook_page') {
        return 'Facebook Page';
    }

    if (key === 'instagram') {
        return 'Instagram';
    }

    if (key === 'mastodon') {
        return 'Mastodon';
    }

    if (key === 'twitter') {
        return 'X';
    }

    if (key === 'tiktok') {
        return 'TikTok';
    }

    return 'Original';
};

const providerCharacterLimit = (provider) => {
    return providerPostRule(provider)?.max_text_chars || null;
};

const previewCharacterLabel = (preview) => {
    const length = preview.body.length;
    const limit = providerCharacterLimit(preview.account.provider);

    return limit ? `${length}/${limit}` : `${length} chars`;
};

const previewOverLimit = (preview) => {
    const limit = providerCharacterLimit(preview.account.provider);

    return Boolean(limit && preview.body.length > limit);
};

const previewHandle = (account) => {
    const username = account.username || account.name || '';

    if (!username || providerKey(account.provider) === 'facebook_page') {
        return providerDisplayName(account.provider);
    }

    return username.startsWith('@') ? username : `@${username}`;
};

const accountInitials = (account) => {
    const label = account.name || account.username || providerDisplayName(account.provider);

    return label
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((part) => part[0]?.toUpperCase())
        .join('') || 'DW';
};

const csvCell = (value) => {
    const text = String(value ?? '');

    if (/[",\n\r]/.test(text)) {
        return `"${text.replace(/"/g, '""')}"`;
    }

    return text;
};

const csvRow = (values) => {
    return values.map(csvCell).join(',');
};

const accountOnboardingTemplateText = () => {
    const rows = [
        ['provider', 'display_name', 'handle_or_page_id', 'owner', 'posting_allowed', 'import_history', 'notes'],
        ['twitter', '', '', '', 'yes', 'yes', ''],
        ['facebook_page', '', '', '', 'yes', 'yes', ''],
        ['instagram', '', '', '', 'yes', 'yes', 'Use Facebook OAuth, then choose connected Instagram accounts.'],
        ['mastodon', '', '', '', 'yes', 'yes', ''],
        ['tiktok', '', '', '', 'assisted', 'yes', 'Requires broker-issued connection credential for analytics.'],
    ];

    return rows.map(csvRow).join('\n');
};

const connectedAccountInventoryText = () => {
    const rows = [
        ['provider', 'display_name', 'handle_or_page_id', 'provider_id', 'authorized', 'import_supported', 'uuid'],
        ...snapshot.value.accounts.map((account) => [
            account.provider,
            account.name || '',
            previewHandle(account) || account.username || '',
            account.provider_id || '',
            account.authorized ? 'yes' : 'no',
            accountImportSupported(account) ? 'yes' : 'no',
            account.uuid,
        ]),
    ];

    return rows.map(csvRow).join('\n');
};

const accountOnboardingPlanText = () => {
    const providerCounts = snapshot.value.accounts.reduce((counts, account) => {
        counts[account.provider] = (counts[account.provider] || 0) + 1;

        return counts;
    }, {});
    const providerSummary = ['twitter', 'facebook_page', 'instagram', 'mastodon', 'tiktok']
        .map((provider) => `${provider}: ${providerCounts[provider] || 0}`)
        .join(', ');
    const readyServices = serviceDefinitions
        .map((service) => `${service.id}: ${serviceReady(service.id) ? 'ready' : 'needs setup'}`)
        .join(', ');
    const lines = [
        'Dust Wave Social account onboarding',
        `Generated: ${new Date().toLocaleString()}`,
        `Connected accounts: ${snapshot.value.accounts.length}`,
        `Import-ready accounts: ${connectedImportAccountCount.value}`,
        `Provider counts: ${providerSummary}`,
        `Service readiness: ${readyServices}`,
        '',
        'Blank intake CSV',
        accountOnboardingTemplateText(),
        '',
        'Connected account inventory CSV',
        connectedAccountInventoryText(),
        '',
        'Next actions',
        '1. Fill the blank intake CSV with every Dust Wave account that should be managed.',
        '2. Configure any provider marked needs setup in Connections > Provider setup.',
        '3. Connect each account from Connections > Connected accounts, then refresh and queue imports for supported accounts.',
        '4. Keep unsupported providers in the notes column for future provider work or manual workflow coverage.',
    ];

    return lines.join('\n');
};

const copyAccountOnboardingTemplate = async () => {
    accountError.value = '';

    if (await writeClipboard(accountOnboardingTemplateText(), accountError)) {
        accountOnboardingCopied.value = 'template';
    }
};

const copyAccountOnboardingPlan = async () => {
    accountError.value = '';

    if (await writeClipboard(accountOnboardingPlanText(), accountError)) {
        accountOnboardingCopied.value = 'plan';
    }
};

const decoratePreviewMedia = (item) => ({
    ...item,
    previewUrl: mediaAssetUrl(item?.thumb_url || item?.url || ''),
    fullUrl: mediaAssetUrl(item?.url || item?.thumb_url || ''),
});

const buildProviderPreviewCard = ({
    account,
    body,
    media,
    hasCustomVersion = false,
}) => {
    const card = {
        account,
        providerKey: providerKey(account.provider),
        providerName: providerDisplayName(account.provider),
        name: account.name || account.username || providerDisplayName(account.provider),
        handle: previewHandle(account),
        initials: accountInitials(account),
        avatarUrl: mediaAssetUrl(account.avatar_path || account.image || ''),
        body,
        media: media.map(decoratePreviewMedia),
        hasCustomVersion,
    };

    return {
        ...card,
        characterLabel: previewCharacterLabel(card),
        overLimit: previewOverLimit(card),
    };
};

const orderedDraftAccountIds = (ids) => {
    const selected = new Set(Array.from(ids).map((id) => Number(id)));
    const ordered = snapshot.value.accounts
        .filter((account) => selected.has(Number(account.id)))
        .map((account) => Number(account.id));
    const known = new Set(ordered);
    const remaining = Array.from(selected).filter((id) => !known.has(id));

    return [...ordered, ...remaining];
};

const isDraftAccountSelected = (account) => {
    return draftAccountIds.value.map((id) => Number(id)).includes(Number(account.id));
};

const draftAccountDisabledReason = (account) => {
    if (isDraftAccountSelected(account)) {
        return '';
    }

    const provider = providerKey(account.provider);
    const rule = providerPostRule(provider);

    if (rule && !rule.simultaneous_posting && (selectedDraftProviderCounts.value[provider] || 0) > 0) {
        return `${providerDisplayName(account.provider)} does not allow simultaneous posting to multiple accounts.`;
    }

    return '';
};

const setActiveDraftVersion = (id) => {
    const accountId = Number(id) || 0;
    const isAvailableVersion = draftVersionTabs.value.some((tab) => Number(tab.id) === accountId);

    activeDraftVersion.value = isAvailableVersion ? accountId : 0;
};

const clearDraftAccountOverride = (accountId) => {
    if (!Object.prototype.hasOwnProperty.call(draftAccountBodies.value, accountId)) {
        return;
    }

    const nextBodies = { ...draftAccountBodies.value };
    delete nextBodies[accountId];
    draftAccountBodies.value = nextBodies;
};

const toggleDraftAccount = (account) => {
    const accountId = Number(account.id);
    const selected = new Set(draftAccountIds.value.map((id) => Number(id)));

    if (selected.has(accountId)) {
        selected.delete(accountId);
        clearDraftAccountOverride(accountId);

        if (Number(activeDraftVersion.value) === accountId) {
            activeDraftVersion.value = 0;
        }
    } else {
        const disabledReason = draftAccountDisabledReason(account);

        if (disabledReason) {
            draftError.value = disabledReason;
            return;
        }

        draftError.value = '';
        selected.add(accountId);
    }

    draftAccountIds.value = orderedDraftAccountIds(selected);
};

const createDraftAccountVersion = (accountId) => {
    const numericAccountId = Number(accountId);

    if (!draftAccountIds.value.map((id) => Number(id)).includes(numericAccountId)) {
        return;
    }

    draftAccountBodies.value = {
        ...draftAccountBodies.value,
        [numericAccountId]: draftBody.value,
    };
    activeDraftVersion.value = numericAccountId;
    versionPickerOpen.value = false;
};

const removeDraftAccountVersion = (accountId) => {
    const numericAccountId = Number(accountId);

    clearDraftAccountOverride(numericAccountId);

    if (Number(activeDraftVersion.value) === numericAccountId) {
        activeDraftVersion.value = 0;
    }
};

const orderedDraftMediaIds = (ids) => {
    const selected = new Set(Array.from(ids).map((id) => Number(id)));
    const ordered = mediaLibrary.value
        .filter((item) => selected.has(Number(item.id)))
        .map((item) => Number(item.id));
    const known = new Set(ordered);
    const remaining = Array.from(selected).filter((id) => !known.has(id));

    return [...ordered, ...remaining];
};

const isDraftMediaSelected = (item) => {
    return draftMediaIds.value.map((id) => Number(id)).includes(Number(item.id));
};

const toggleDraftMedia = (item) => {
    const mediaId = Number(item.id);
    const selected = new Set(draftMediaIds.value.map((id) => Number(id)));

    if (selected.has(mediaId)) {
        selected.delete(mediaId);
    } else {
        selected.add(mediaId);
    }

    draftMediaIds.value = orderedDraftMediaIds(selected);
};

const removeDraftMedia = (mediaId) => {
    draftMediaIds.value = draftMediaIds.value.filter((id) => Number(id) !== Number(mediaId));
};

const removeDraftExternalMedia = (mediaId) => {
    draftExternalMedia.value = draftExternalMedia.value.filter((item) => item.id !== mediaId);
};

const selectDraftTag = (tag) => {
    const tagId = Number(tag.id);

    if (!draftTagIds.value.map((id) => Number(id)).includes(tagId)) {
        draftTagIds.value = [...draftTagIds.value.map((id) => Number(id)), tagId];
    }

    tagSearchText.value = '';
};

const removeDraftTag = (tagId) => {
    draftTagIds.value = draftTagIds.value.filter((id) => Number(id) !== Number(tagId));
};

const pickDraftTagColor = () => {
    const palette = COLOR_PALLET_LIST().map((color) => color.replace('#', '').toLowerCase());
    const used = new Set(snapshot.value.tags.map((tag) => String(tag.hex_color || '').replace('#', '').toLowerCase()));

    return palette.find((color) => !used.has(color)) || palette[0] || '101215';
};

const createDraftTag = async () => {
    const name = tagSearchText.value.trim();

    if (!name || tagSaving.value) {
        return;
    }

    const existing = snapshot.value.tags.find((tag) => tag.name.toLowerCase() === name.toLowerCase());

    if (existing) {
        selectDraftTag(existing);
        return;
    }

    tagSaving.value = true;
    tagError.value = '';

    try {
        const tag = await invoke('create_tag', {
            tag: {
                name,
                hex_color: pickDraftTagColor(),
            },
        });

        selectDraftTag(tag);
        await load();
    } catch (error) {
        tagError.value = String(error);
    } finally {
        tagSaving.value = false;
    }
};

const draftEditor = useTipTapEditor({
    content: normalizeEditorContent(activeDraftBody.value),
    extensions: [
        Document,
        Div,
        Text,
        Link.configure({
            openOnClick: false,
            linkOnPaste: false,
        }),
        History,
        Placeholder.configure({
            placeholder: 'Write post copy',
        }),
        Typography.configure({
            openDoubleQuote: false,
            closeDoubleQuote: false,
            openSingleQuote: false,
            closeSingleQuote: false,
        }),
    ],
    editorProps: {
        attributes: {
            class: 'desktop-rich-editor-content',
        },
    },
    onUpdate: ({ editor }) => {
        const html = editor.getHTML();

        if (activeDraftBody.value !== html) {
            activeDraftBody.value = html;
        }
    },
});

const runDraftEditorCommand = (command) => {
    const editor = draftEditor.value;

    if (!editor) {
        return;
    }

    const chain = editor.chain().focus();

    if (command === 'undo') {
        chain.undo().run();
    }

    if (command === 'redo') {
        chain.redo().run();
    }
};

const insertDraftEmoji = (emoji) => {
    const value = typeof emoji === 'string' ? emoji : emoji?.native;

    if (!value) {
        return;
    }

    const editor = draftEditor.value;

    if (editor) {
        editor.chain().focus().insertContent(value).run();
    } else {
        activeDraftBody.value = `${activeDraftBody.value || ''}${value}`;
    }

    emojiPickerOpen.value = false;
};

const canEditPost = (post) => {
    return ['draft', 'scheduled'].includes(post.status);
};

const canSchedulePost = (post) => {
    return post.account_count > 0 && post.status === 'draft';
};

const canRetryPost = (post) => {
    return post.account_count > 0 && post.status === 'failed';
};

const mediaLibraryRequest = () => {
    return {
        keyword: mediaFilter.value.keyword || null,
        media_type: mediaFilter.value.media_type || null,
        limit: 100,
    };
};

const defaultDraftAccountIds = (accounts = snapshot.value.accounts, draft = settingsDraft.value) => {
    const available = new Set((accounts || []).map((account) => Number(account.id)));

    return (draft.default_accounts || [])
        .map((id) => Number(id))
        .filter((id) => available.has(id));
};

const fetchMediaLibrary = () => {
    return invoke('query_media_library', {
        request: mediaLibraryRequest(),
    });
};

const pickFilePath = async (filters) => {
    const selected = await open({
        multiple: false,
        directory: false,
        filters,
    });

    if (Array.isArray(selected)) {
        return selected[0] || '';
    }

    return selected || '';
};

const pickFilePaths = async (filters) => {
    const selected = await open({
        multiple: true,
        directory: false,
        filters,
    });

    if (Array.isArray(selected)) {
        return selected;
    }

    return selected ? [selected] : [];
};

const pickDirectoryPath = async () => {
    const selected = await open({
        multiple: false,
        directory: true,
    });

    if (Array.isArray(selected)) {
        return selected[0] || '';
    }

    return selected || '';
};

const openOAuthUrl = async (url, errorTarget) => {
    errorTarget.value = '';

    if (!url || !/^https?:\/\//i.test(url)) {
        errorTarget.value = 'Authorization URL is not available';
        return;
    }

    try {
        await openUrl(url);
    } catch (error) {
        errorTarget.value = String(error);
    }
};

const chooseMediaImportSource = async () => {
    mediaError.value = '';

    try {
        const sourcePath = await pickFilePath(dustWaveMediaFileFilters);

        if (sourcePath) {
            mediaImport.value.source_path = sourcePath;
        }
    } catch (error) {
        mediaError.value = String(error);
    }
};

const importMultipleMediaFiles = async () => {
    mediaError.value = '';
    mediaImportResults.value = [];

    try {
        const paths = await pickFilePaths(dustWaveMediaFileFilters);

        if (!paths.length) {
            return;
        }

        mediaSaving.value = true;
        const results = [];

        for (const [index, sourcePath] of paths.entries()) {
            mediaProgress.value = `Importing ${index + 1} of ${paths.length}`;
            try {
                const media = await invoke('import_media_file', {
                    media: {
                        source_path: sourcePath,
                        name: null,
                    },
                });

                results.push({
                    path: sourcePath,
                    name: media.name || fileNameFromPath(sourcePath),
                    status: 'imported',
                    detail: media.mime_type,
                });
            } catch (error) {
                results.push({
                    path: sourcePath,
                    name: fileNameFromPath(sourcePath),
                    status: 'failed',
                    detail: String(error),
                });
            }
        }

        mediaImportResults.value = results;
        const failed = results.filter((result) => result.status === 'failed');

        if (failed.length) {
            mediaError.value = `${failed.length} file(s) could not be imported`;
        }

        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const importMediaForDraft = async () => {
    mediaError.value = '';
    mediaImportResults.value = [];

    try {
        const paths = await pickFilePaths(dustWaveMediaFileFilters);

        if (!paths.length) {
            return;
        }

        mediaSaving.value = true;
        const importedIds = [];
        const results = [];

        for (const [index, sourcePath] of paths.entries()) {
            mediaProgress.value = `Importing ${index + 1} of ${paths.length}`;
            try {
                const media = await invoke('import_media_file', {
                    media: {
                        source_path: sourcePath,
                        name: null,
                    },
                });

                importedIds.push(Number(media.id));
                results.push({
                    path: sourcePath,
                    name: media.name || fileNameFromPath(sourcePath),
                    status: 'imported',
                    detail: media.mime_type,
                });
            } catch (error) {
                results.push({
                    path: sourcePath,
                    name: fileNameFromPath(sourcePath),
                    status: 'failed',
                    detail: String(error),
                });
            }
        }

        mediaImportResults.value = results;
        const failed = results.filter((result) => result.status === 'failed');

        if (failed.length) {
            mediaError.value = `${failed.length} file(s) could not be imported`;
        }

        await load();
        draftMediaIds.value = orderedDraftMediaIds(new Set([
            ...draftMediaIds.value.map((id) => Number(id)),
            ...importedIds,
        ]));
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const loadMediaLibrary = async () => {
    mediaLibraryLoading.value = true;
    mediaLibraryError.value = '';

    try {
        mediaLibrary.value = await fetchMediaLibrary();
    } catch (error) {
        mediaLibrary.value = [];
        mediaLibraryError.value = String(error);
    } finally {
        mediaLibraryLoading.value = false;
    }
};

const externalMediaSourceLabel = (source) => {
    return source === 'gifs' ? 'Klipy' : 'Unsplash';
};

const externalMediaProvider = (item = null) => String(item?.download_data?.provider || '').toLowerCase();

const isKlipyExternalMedia = (item = null, source = externalMediaResults.value?.source || externalMediaSearch.value.source) => {
    const provider = externalMediaProvider(item);

    return provider === 'klipy' || source === 'gifs';
};

const selectedExternalMediaIncludesKlipy = computed(() => {
    if (activeMediaTab.value === 'uploads') {
        return false;
    }

    return selectedExternalMediaItems.value.some((item) => isKlipyExternalMedia(item, activeMediaTab.value));
});

const selectedExternalMediaPolicyNote = computed(() => (
    selectedExternalMediaIncludesKlipy.value
        ? 'Klipy GIFs attach as provider references and are fetched temporarily only while publishing.'
        : ''
));

const canDownloadExternalMediaItem = (item) => !isKlipyExternalMedia(
    item,
    externalMediaResults.value?.source || externalMediaSearch.value.source,
);

const externalMediaReference = (item) => ({
    id: String(item.id || item.url || '').trim(),
    name: item.name || 'Klipy GIF',
    mime_type: item.mime_type || 'image/gif',
    media_type: item.media_type || 'gif',
    url: item.url,
    thumb_url: item.thumb_url || item.url,
    is_video: Boolean(item.is_video),
    credit_url: item.credit_url || null,
    download_data: {
        ...(item.download_data || {}),
        provider: externalMediaProvider(item) || 'klipy',
    },
});

const externalMediaPreviewResource = (item) => ({
    ...item,
    uuid: `external:${externalMediaProvider(item) || 'provider'}:${item.id}`,
    source_label: externalMediaProvider(item) === 'klipy' ? 'Klipy reference' : 'External reference',
    size_total: 0,
    external: true,
});

const externalMediaPreviewResources = (items = []) => items.map(externalMediaPreviewResource);

const searchExternalMedia = async (page = externalMediaSearch.value.page) => {
    externalMediaLoading.value = true;
    externalMediaError.value = '';
    externalMediaSearch.value.page = Number(page) || 1;

    try {
        externalMediaResults.value = await invoke('search_external_media', {
            request: {
                source: externalMediaSearch.value.source,
                keyword: externalMediaSearch.value.keyword || null,
                page: externalMediaSearch.value.page,
                limit: 18,
            },
        });
        externalMediaSearch.value.page = externalMediaResults.value.page;
    } catch (error) {
        externalMediaResults.value = null;
        externalMediaError.value = String(error);
    } finally {
        externalMediaLoading.value = false;
    }
};

const searchNextExternalMediaPage = () => {
    searchExternalMedia(externalMediaResults.value?.next_page || externalMediaSearch.value.page + 1);
};

const localAiCancelError = () => {
    const error = new Error('Local AI operation canceled');
    error.name = 'AbortError';
    return error;
};

const throwIfLocalAiCanceled = (signal) => {
    if (signal?.aborted) {
        throw localAiCancelError();
    }
};

const ensureLocalAiLiteRt = async () => {
    const litert = await import('@litertjs/core');

    if (!localAiLiteRtLoaded) {
        await litert.loadLiteRt('./litert/wasm/');
        localAiLiteRtLoaded = true;
    }

    return litert;
};

const localAiModelShapeValue = (shape, index, fallback) => {
    const value = Number(Array.isArray(shape) ? shape[index] : fallback);
    return Number.isFinite(value) && value > 0 ? value : fallback;
};

const loadLocalAiUpscalingBundle = async () => {
    const manifestResponse = await fetch('./litert/models/manifest.json', { cache: 'no-store' });

    if (!manifestResponse.ok) {
        throw new Error(`Local AI model manifest unavailable (${manifestResponse.status})`);
    }

    const manifest = await manifestResponse.json();
    const models = Array.isArray(manifest?.models) ? manifest.models : [];
    const upscalingModel = models.find((model) => model.feature === 'image_upscaling' && model.runtime === 'litert');
    const modelFile = upscalingModel?.files?.find((file) => file.kind === 'model');

    if (!upscalingModel || !modelFile?.path) {
        throw new Error('Local AI upscaling model is not listed in the bundled manifest');
    }

    const inputShape = upscalingModel.inputs?.image?.shape || [1, LOCAL_AI_UPSCALE_TILE_SIZE, LOCAL_AI_UPSCALE_TILE_SIZE, 3];
    const outputShape = upscalingModel.outputs?.upscaled_image?.shape || [
        1,
        LOCAL_AI_UPSCALE_TILE_SIZE * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        LOCAL_AI_UPSCALE_TILE_SIZE * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        3,
    ];
    const inputTileSize = localAiModelShapeValue(inputShape, 1, LOCAL_AI_UPSCALE_TILE_SIZE);
    const outputTileSize = localAiModelShapeValue(outputShape, 1, inputTileSize * LOCAL_AI_UPSCALE_SCALE_FACTOR);
    const modelScaleFactor = outputTileSize / inputTileSize;

    if (inputTileSize !== LOCAL_AI_UPSCALE_TILE_SIZE || modelScaleFactor !== LOCAL_AI_UPSCALE_SCALE_FACTOR) {
        throw new Error('Bundled Local AI upscaling model must use 128px input tiles and x4 output');
    }

    return {
        manifest,
        models,
        upscalingModel,
        modelFile,
        inputShape,
        outputShape,
        inputTileSize,
        outputTileSize,
        modelScaleFactor,
    };
};

const compileLocalAiUpscalingModel = async () => {
    const webgpu = Boolean(navigator.gpu);
    const bundle = await loadLocalAiUpscalingBundle();
    const { loadAndCompile, Tensor } = await ensureLocalAiLiteRt();
    const compiledModel = await loadAndCompile(`./litert/models/${bundle.modelFile.path}`, {
        accelerator: webgpu ? ['webgpu', 'wasm'] : 'wasm',
    });
    const accelerator = webgpu ? (compiledModel.isFullyAccelerated ? 'webgpu' : 'wasm_fallback') : 'wasm';

    return {
        ...bundle,
        Tensor,
        compiledModel,
        webgpu,
        accelerator,
    };
};

const createLocalAiCanvas = (width, height) => {
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(width));
    canvas.height = Math.max(1, Math.round(height));
    return canvas;
};

const localAiCanvasContext = (canvas) => {
    const context = canvas.getContext('2d', { willReadFrequently: true });

    if (!context) {
        throw new Error('Local AI canvas context unavailable');
    }

    return context;
};

const loadLocalAiImage = async (url) => new Promise((resolve, reject) => {
    const image = new Image();
    image.decoding = 'async';
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('Local AI source image could not be loaded'));
    image.src = url;
});

const prepareLocalAiSourceCanvas = async (item) => {
    const sourceUrl = mediaAssetUrl(item?.url || item?.thumb_url || '');

    if (!sourceUrl) {
        throw new Error('Local AI source image URL is unavailable');
    }

    const image = await loadLocalAiImage(sourceUrl);
    const originalWidth = image.naturalWidth || image.width;
    const originalHeight = image.naturalHeight || image.height;

    if (!originalWidth || !originalHeight) {
        throw new Error('Local AI source image dimensions are unavailable');
    }

    const processingScale = Math.min(1, LOCAL_AI_UPSCALE_MAX_SOURCE_EDGE / Math.max(originalWidth, originalHeight));
    const processingWidth = Math.max(1, Math.round(originalWidth * processingScale));
    const processingHeight = Math.max(1, Math.round(originalHeight * processingScale));
    const canvas = createLocalAiCanvas(processingWidth, processingHeight);
    const context = localAiCanvasContext(canvas);
    context.drawImage(image, 0, 0, processingWidth, processingHeight);

    return {
        canvas,
        context,
        originalWidth,
        originalHeight,
        processingWidth,
        processingHeight,
    };
};

const localAiTileInput = (sourceContext, tileX, tileY, cropWidth, cropHeight) => {
    const pixels = sourceContext.getImageData(tileX, tileY, cropWidth, cropHeight).data;
    const input = new Uint8Array(LOCAL_AI_UPSCALE_TILE_SIZE * LOCAL_AI_UPSCALE_TILE_SIZE * 3);

    for (let y = 0; y < LOCAL_AI_UPSCALE_TILE_SIZE; y += 1) {
        const sourceY = Math.min(y, cropHeight - 1);

        for (let x = 0; x < LOCAL_AI_UPSCALE_TILE_SIZE; x += 1) {
            const sourceX = Math.min(x, cropWidth - 1);
            const sourceIndex = (sourceY * cropWidth + sourceX) * 4;
            const inputIndex = (y * LOCAL_AI_UPSCALE_TILE_SIZE + x) * 3;
            input[inputIndex] = pixels[sourceIndex];
            input[inputIndex + 1] = pixels[sourceIndex + 1];
            input[inputIndex + 2] = pixels[sourceIndex + 2];
        }
    }

    return input;
};

const localAiModelByte = (value) => {
    const scaled = value >= 0 && value <= 1 ? value * 255 : value;
    return Math.max(0, Math.min(255, Math.round(scaled)));
};

const drawLocalAiUpscaleTile = (targetContext, outputData, tileX, tileY, cropWidth, cropHeight, outputTileSize) => {
    const tileCanvas = createLocalAiCanvas(outputTileSize, outputTileSize);
    const tileContext = localAiCanvasContext(tileCanvas);
    const imageData = tileContext.createImageData(outputTileSize, outputTileSize);

    for (let outputIndex = 0, rgbaIndex = 0; outputIndex < outputData.length; outputIndex += 3, rgbaIndex += 4) {
        imageData.data[rgbaIndex] = localAiModelByte(outputData[outputIndex]);
        imageData.data[rgbaIndex + 1] = localAiModelByte(outputData[outputIndex + 1]);
        imageData.data[rgbaIndex + 2] = localAiModelByte(outputData[outputIndex + 2]);
        imageData.data[rgbaIndex + 3] = 255;
    }

    tileContext.putImageData(imageData, 0, 0);
    targetContext.drawImage(
        tileCanvas,
        0,
        0,
        cropWidth * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        cropHeight * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        tileX * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        tileY * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        cropWidth * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        cropHeight * LOCAL_AI_UPSCALE_SCALE_FACTOR,
    );
};

const runLocalAiUpscaleTile = async (compiledModel, Tensor, inputBytes, inputShape) => {
    let inputTensor = null;
    let outputTensor = null;
    let cpuOutputTensor = null;
    let outputs = null;

    try {
        inputTensor = new Tensor(inputBytes, inputShape);
        outputs = await compiledModel.run(inputTensor);
        const outputList = Array.isArray(outputs) ? outputs : Object.values(outputs || {});
        outputTensor = outputList[0];

        if (!outputTensor) {
            throw new Error('Local AI model did not return an output tensor');
        }

        cpuOutputTensor = outputTensor.accelerator === 'wasm' ? outputTensor : await outputTensor.moveTo('wasm');
        const outputData = cpuOutputTensor.toTypedArray();
        return outputData.slice ? outputData.slice() : new outputData.constructor(outputData);
    } finally {
        if (inputTensor && !inputTensor.deleted) {
            inputTensor.delete();
        }

        const outputList = Array.isArray(outputs) ? outputs : Object.values(outputs || {});

        for (const tensor of outputList) {
            if (tensor && tensor !== cpuOutputTensor && !tensor.deleted) {
                tensor.delete();
            }
        }

        if (cpuOutputTensor && !cpuOutputTensor.deleted) {
            cpuOutputTensor.delete();
        }
    }
};

const upscaleLocalAiMediaWithModel = async (item, { signal, setProgress }) => {
    let compiledModel = null;

    try {
        throwIfLocalAiCanceled(signal);
        setProgress?.('Loading bundled upscaling model');
        const modelBundle = await compileLocalAiUpscalingModel();
        compiledModel = modelBundle.compiledModel;

        throwIfLocalAiCanceled(signal);
        setProgress?.('Preparing source image');
        const source = await prepareLocalAiSourceCanvas(item);
        const outputCanvas = createLocalAiCanvas(
            source.processingWidth * LOCAL_AI_UPSCALE_SCALE_FACTOR,
            source.processingHeight * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        );
        const outputContext = localAiCanvasContext(outputCanvas);
        const tileColumns = Math.ceil(source.processingWidth / LOCAL_AI_UPSCALE_TILE_SIZE);
        const tileRows = Math.ceil(source.processingHeight / LOCAL_AI_UPSCALE_TILE_SIZE);
        const tileTotal = tileColumns * tileRows;
        let tileIndex = 0;

        for (let row = 0; row < tileRows; row += 1) {
            for (let column = 0; column < tileColumns; column += 1) {
                throwIfLocalAiCanceled(signal);
                tileIndex += 1;
                setProgress?.(`Upscaling tile ${tileIndex} of ${tileTotal}`);

                const tileX = column * LOCAL_AI_UPSCALE_TILE_SIZE;
                const tileY = row * LOCAL_AI_UPSCALE_TILE_SIZE;
                const cropWidth = Math.min(LOCAL_AI_UPSCALE_TILE_SIZE, source.processingWidth - tileX);
                const cropHeight = Math.min(LOCAL_AI_UPSCALE_TILE_SIZE, source.processingHeight - tileY);
                const inputBytes = localAiTileInput(source.context, tileX, tileY, cropWidth, cropHeight);
                const outputData = await runLocalAiUpscaleTile(
                    compiledModel,
                    modelBundle.Tensor,
                    inputBytes,
                    modelBundle.inputShape,
                );
                drawLocalAiUpscaleTile(
                    outputContext,
                    outputData,
                    tileX,
                    tileY,
                    cropWidth,
                    cropHeight,
                    modelBundle.outputTileSize,
                );
            }
        }

        throwIfLocalAiCanceled(signal);
        setProgress?.('Saving derivative');
        const imageDataUrl = outputCanvas.toDataURL('image/png');

        return invoke('save_local_ai_model_upscale_derivative', {
            form: {
                uuid: item.uuid,
                image_data_url: imageDataUrl,
                scale_factor: LOCAL_AI_UPSCALE_SCALE_FACTOR,
                model_id: modelBundle.upscalingModel.id,
                model_name: modelBundle.upscalingModel.name,
                model_version: modelBundle.upscalingModel.version,
                model_license: modelBundle.upscalingModel.license,
                model_source_url: modelBundle.upscalingModel.source_url,
                model_license_url: modelBundle.upscalingModel.license_url,
                model_precision: modelBundle.upscalingModel.precision,
                accelerator: modelBundle.accelerator,
                tile_size: LOCAL_AI_UPSCALE_TILE_SIZE,
                input_width: source.processingWidth,
                input_height: source.processingHeight,
                original_width: source.originalWidth,
                original_height: source.originalHeight,
            },
        });
    } finally {
        if (compiledModel && !compiledModel.deleted) {
            compiledModel.delete();
        }
    }
};

const probeLocalAiRuntime = async () => {
    localAiBusy.value = true;
    localAiError.value = '';
    localAiResult.value = null;
    const webgpu = Boolean(navigator.gpu);
    let compiledModel = null;

    try {
        const modelBundle = await compileLocalAiUpscalingModel();
        compiledModel = modelBundle.compiledModel;

        localAiRuntime.value = {
            status: 'ready',
            accelerator: modelBundle.accelerator === 'webgpu' ? 'webgpu_model_ready' : `${modelBundle.accelerator}_model_ready`,
            webgpu,
            wasm: true,
            models: modelBundle.models.length,
            compiled_model: modelBundle.upscalingModel.name,
            detail: webgpu
                ? `WebGPU detected; LiteRT Wasm loaded; ${modelBundle.upscalingModel.name} compiled`
                : `LiteRT Wasm loaded; ${modelBundle.upscalingModel.name} compiled`,
        };
    } catch (error) {
        localAiRuntime.value = {
            status: 'error',
            accelerator: webgpu ? 'webgpu_available_wasm_error' : 'wasm_error',
            webgpu,
            wasm: false,
            models: 0,
            compiled_model: '',
            detail: String(error),
        };
        localAiError.value = String(error);
    } finally {
        if (compiledModel && !compiledModel.deleted) {
            compiledModel.delete();
        }

        localAiBusy.value = false;
    }
};

const cancelLocalAiOperation = () => {
    if (!localAiAbortController) {
        return;
    }

    localAiAbortController.abort();
    localAiResult.value = {
        operation: localAiResult.value?.operation || 'Local AI operation',
        status: 'canceling',
        detail: 'Canceling after the current tile',
    };
};

const runLocalAiMediaAction = async (operation, action, { reload = false } = {}) => {
    const controller = new AbortController();
    localAiAbortController = controller;
    localAiBusy.value = true;
    localAiError.value = '';
    localAiResult.value = { operation, status: 'running', detail: 'Starting' };

    const setProgress = (detail) => {
        if (!controller.signal.aborted) {
            localAiResult.value = { operation, status: 'running', detail };
        }
    };

    try {
        const result = await action({ signal: controller.signal, setProgress });
        throwIfLocalAiCanceled(controller.signal);
        localAiResult.value = { operation, status: 'complete', result };

        if (reload) {
            await load();
            await loadMediaLibrary();
        }
    } catch (error) {
        if (controller.signal.aborted || error?.name === 'AbortError') {
            localAiResult.value = { operation, status: 'canceled', detail: 'Canceled' };
        } else {
            localAiResult.value = null;
            localAiError.value = String(error);
        }
    } finally {
        if (localAiAbortController === controller) {
            localAiAbortController = null;
        }

        localAiBusy.value = false;
    }
};

const preflightLocalAiMedia = async (item) => {
    await runLocalAiMediaAction('Media quality preflight', () => invoke('local_ai_preflight_media', { uuid: item.uuid }));
};

const upscaleLocalAiMedia = async (item, scaleFactor = LOCAL_AI_UPSCALE_SCALE_FACTOR) => {
    if (scaleFactor !== LOCAL_AI_UPSCALE_SCALE_FACTOR) {
        await runLocalAiMediaAction(
            'Local image upscaling',
            () => invoke('create_local_ai_upscale_derivative', { uuid: item.uuid, scaleFactor }),
            { reload: true },
        );
        return;
    }

    await runLocalAiMediaAction(
        'Local image upscaling',
        ({ signal, setProgress }) => upscaleLocalAiMediaWithModel(item, { signal, setProgress }),
        { reload: true },
    );
};

const cropLocalAiMedia = async (item) => {
    await runLocalAiMediaAction(
        'Smart crop suggestion',
        () => invoke('create_local_ai_crop_derivative', { uuid: item.uuid, targetRatio: localAiCropRatio.value }),
        { reload: true },
    );
};

const draftLocalAiAltText = async (item) => {
    await runLocalAiMediaAction('Alt-text draft', () => invoke('draft_local_ai_alt_text', { uuid: item.uuid }));
};

const searchLocalAiMedia = async () => {
    localAiBusy.value = true;
    localAiError.value = '';
    localAiResult.value = null;

    try {
        localAiSearchResults.value = await invoke('local_ai_media_search', {
            query: localAiSearchQuery.value,
        });
    } catch (error) {
        localAiSearchResults.value = null;
        localAiError.value = String(error);
    } finally {
        localAiBusy.value = false;
    }
};

const setMediaTab = (tab) => {
    activeMediaTab.value = tab;
    externalMediaError.value = '';

    if (tab === 'stock' || tab === 'gifs') {
        externalMediaSearch.value.source = tab;
        externalMediaResults.value = null;
    }
};

const syncServiceConfigurationDrafts = (services = snapshot.value.services) => {
    const nextDrafts = JSON.parse(JSON.stringify(serviceConfigurationDefaults));
    const nextActiveDrafts = { ...serviceActiveDraftDefaults };

    serviceDefinitions.forEach((definition) => {
        const service = services.find((item) => item.name === definition.id);
        nextDrafts[definition.id] = {
            ...(nextDrafts[definition.id] || {}),
            ...(service?.configuration || {}),
        };
        nextActiveDrafts[definition.id] = Boolean(
            service?.active
            ?? serviceStatusByName(definition.id)?.active
            ?? false,
        );
    });

    serviceConfigurationDrafts.value = nextDrafts;
    serviceActiveDrafts.value = nextActiveDrafts;
};

const draftSubmitLabel = computed(() => {
    return editingPostUuid.value ? 'Update Post' : 'Save Draft';
});

const draftScheduleDate = computed(() => {
    if (!draftScheduledAt.value) {
        return null;
    }

    const date = new Date(draftScheduledAt.value);

    return Number.isNaN(date.getTime()) ? null : date;
});

const draftScheduleLabel = computed(() => {
    if (!draftScheduledAt.value) {
        return 'Pick time';
    }

    if (!draftScheduleDate.value) {
        return 'Invalid time';
    }

    const datePart = draftScheduleDate.value.toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric',
    });
    const timePart = draftScheduleDate.value.toLocaleTimeString(undefined, {
        hour: 'numeric',
        minute: '2-digit',
        hour12: Number(settings.value?.time_format ?? 12) !== 24,
    });

    return `${datePart}, ${timePart}`;
});

const canSaveDraft = computed(() => {
    return Boolean(
        compactEditorText(draftBody.value)
        || draftMediaIds.value.length
        || draftExternalMedia.value.length,
    );
});

const canScheduleDraft = computed(() => {
    return Boolean(canSaveDraft.value && draftAccountIds.value.length && (!draftScheduledAt.value || draftScheduleDate.value));
});

const ensureNotificationPermission = async () => {
    if (notificationPermissionRequest) {
        return notificationPermissionRequest;
    }

    notificationPermissionRequest = (async () => {
        try {
            desktopNotificationError.value = '';

            if (await isPermissionGranted()) {
                return true;
            }

            const permission = await requestPermission();
            const granted = permission === 'granted';

            if (!granted) {
                desktopNotificationError.value = 'Desktop notifications are not enabled';
            }

            return granted;
        } catch (error) {
            desktopNotificationError.value = String(error);

            return false;
        } finally {
            notificationPermissionRequest = null;
        }
    })();

    return notificationPermissionRequest;
};

const sendDesktopNotification = async (title, body, options = {}) => {
    if (!options.force && settings.value?.desktop_notifications === false) {
        return false;
    }

    if (!(await ensureNotificationPermission())) {
        return false;
    }

    try {
        sendNotification({
            title,
            body,
        });

        return true;
    } catch (error) {
        desktopNotificationError.value = String(error);

        return false;
    }
};

const notifyHealthIssues = async (nextHealth) => {
    if (settings.value?.desktop_notifications === false) {
        healthNotificationKey.value = '';
        return;
    }

    const counts = nextHealth?.counts || {};
    const issues = [
        {
            key: 'unauthorized_accounts',
            title: 'Account connection lost',
            count: Number(counts.unauthorized_accounts) || 0,
            body: `${pluralize(counts.unauthorized_accounts, 'account')} ${Number(counts.unauthorized_accounts) === 1 ? 'needs' : 'need'} reconnection before publishing or importing.`,
        },
        {
            key: 'failed_posts',
            title: 'Failed posts need review',
            count: Number(counts.failed_posts) || 0,
            body: `${pluralize(counts.failed_posts, 'post')} failed and should be reviewed or retried.`,
        },
        {
            key: 'failed_jobs',
            title: 'Background work failed',
            count: Number(counts.failed_jobs) || 0,
            body: `${pluralize(counts.failed_jobs, 'background item')} failed. Review the system log before clearing resolved state.`,
        },
        {
            key: 'rate_limits',
            title: 'Provider rate limit active',
            count: Number(counts.rate_limits) || 0,
            body: `${pluralize(counts.rate_limits, 'provider limit')} ${Number(counts.rate_limits) === 1 ? 'is' : 'are'} active. Publishing may resume after the limit clears.`,
        },
    ].filter((issue) => issue.count > 0);
    const nextKey = issues.map((issue) => `${issue.key}:${issue.count}`).join('|');

    if (!nextKey) {
        healthNotificationKey.value = '';
        return;
    }

    if (nextKey === healthNotificationKey.value) {
        return;
    }

    healthNotificationKey.value = nextKey;

    const primaryIssue = issues[0];
    const body = issues.length === 1
        ? primaryIssue.body
        : `${primaryIssue.body} ${pluralize(issues.length - 1, 'more area')} need review.`;

    await sendDesktopNotification(primaryIssue.title, body);
};

const notifyWorkerRun = async (summary) => {
    const completed = Number(summary?.completed) || 0;
    const failed = Number(summary?.failed) || 0;

    if (completed === 0 && failed === 0) {
        return;
    }

    if (failed > 0) {
        const completedBody = completed > 0 ? ` ${pluralize(completed, 'item')} published.` : '';
        await sendDesktopNotification(
            'Scheduled publishing needs review',
            `${pluralize(failed, 'item')} failed during background publishing.${completedBody}`,
        );
        return;
    }

    await sendDesktopNotification(
        'Scheduled posts published',
        `${pluralize(completed, 'post')} published from the local queue.`,
    );
};

const sendTestNotification = async () => {
    desktopNotificationError.value = '';
    desktopNotificationTestSent.value = false;

    if (!settingsDraft.value.desktop_notifications) {
        desktopNotificationError.value = 'Desktop notifications are disabled';
        return;
    }

    desktopNotificationTestSent.value = await sendDesktopNotification(
        'Dust Wave Social',
        'Desktop notifications are working.',
        { force: true },
    );
};

const load = async () => {
    try {
        const [
            dashboardResult,
            healthResult,
            settingsResult,
            snapshotResult,
            credentialResult,
            mediaLibraryResult,
            systemLogResult,
        ] = await Promise.all([
            invoke('dashboard_summary'),
            invoke('system_health'),
            invoke('settings'),
            invoke('local_data_snapshot'),
            invoke('service_credential_statuses'),
            fetchMediaLibrary(),
            invoke('system_logs'),
        ]);

        dashboard.value = dashboardResult;
        health.value = healthResult;
        settings.value = settingsResult;
        settingsDraft.value = {
            ...settingsResult,
            default_accounts: [...(settingsResult.default_accounts || [])],
        };
        snapshot.value = snapshotResult;
        credentialStatuses.value = credentialResult;
        mediaLibrary.value = mediaLibraryResult;
        systemLogs.value = systemLogResult;
        syncServiceConfigurationDrafts(snapshotResult.services || []);

        const reportAccountExists = snapshotResult.accounts.some((account) => {
            return Number(account.id) === Number(reportAccountId.value);
        });
        reportAccountId.value = reportAccountExists
            ? Number(reportAccountId.value)
            : snapshotResult.accounts[0]?.id || '';

        if (
            !editingPostUuid.value
            && !compactEditorText(draftBody.value)
            && !draftAccountIds.value.length
            && !draftMediaIds.value.length
            && !draftExternalMedia.value.length
            && !draftTagIds.value.length
            && !draftScheduledAt.value
        ) {
            draftAccountIds.value = defaultDraftAccountIds(snapshotResult.accounts, settingsResult);
        }

        await Promise.all([
            loadReport(reportAccountId.value),
            loadPostQuery(),
        ]);
        await notifyHealthIssues(healthResult);
    } catch (error) {
        loadError.value = String(error);
    }
};

const loadPostQuery = async () => {
    postQueryLoading.value = true;
    postQueryError.value = '';
    const useCalendarWindow = activeView.value === 'calendar';

    try {
        postQuery.value = await invoke('query_posts', {
            request: {
                status: postFilter.value.status || null,
                exclude_status: useCalendarWindow && !postFilter.value.status ? 'draft' : null,
                keyword: postFilter.value.keyword || null,
                accounts: postFilter.value.accounts,
                tags: postFilter.value.tags,
                calendar_type: useCalendarWindow ? postFilter.value.calendar_type : null,
                date: useCalendarWindow ? postFilter.value.date : null,
                limit: useCalendarWindow ? 200 : postFilter.value.per_page,
                page: useCalendarWindow ? 1 : postFilter.value.page,
            },
        });
        postFilter.value.page = postQuery.value.page || 1;
        const visible = new Set(postQuery.value.items.map((post) => post.uuid));
        selectedPostUuids.value = selectedPostUuids.value.filter((uuid) => visible.has(uuid));
    } catch (error) {
        postQuery.value = {
            items: [],
            total: 0,
            page: 1,
            per_page: postFilter.value.per_page,
            total_pages: 1,
            has_failed_posts: false,
            calendar_window: null,
        };
        postQueryError.value = String(error);
    } finally {
        postQueryLoading.value = false;
    }
};

const setPostStatusFilter = async (status) => {
    postFilter.value.status = status;
    postFilter.value.page = 1;
    await loadPostQuery();
};

const applyPostFilters = async () => {
    postFilter.value.page = 1;
    postFilterOpen.value = false;
    await loadPostQuery();
};

const clearPostFilters = async () => {
    postFilter.value.keyword = '';
    postFilter.value.accounts = [];
    postFilter.value.tags = [];
    postFilter.value.page = 1;
    postFilterOpen.value = false;
    await loadPostQuery();
};

const postFilterAccountSelected = (account) => {
    return postFilter.value.accounts.map((id) => Number(id)).includes(Number(account.id));
};

const postFilterTagSelected = (tag) => {
    return postFilter.value.tags.map((id) => Number(id)).includes(Number(tag.id));
};

const changePostPage = async (page) => {
    const nextPage = Math.max(1, Math.min(Number(page) || 1, postQuery.value.total_pages || 1));

    if (nextPage === postFilter.value.page && nextPage === postQuery.value.page) {
        return;
    }

    postFilter.value.page = nextPage;
    await loadPostQuery();
};

const toggleVisiblePostSelection = () => {
    const visible = visiblePostUuids.value;

    if (allVisiblePostsSelected.value) {
        selectedPostUuids.value = selectedPostUuids.value.filter((uuid) => !visible.includes(uuid));
        return;
    }

    selectedPostUuids.value = Array.from(new Set([...selectedPostUuids.value, ...visible]));
};

const clearPostSelection = () => {
    selectedPostUuids.value = [];
};

const selectCalendarDate = async (date) => {
    postFilter.value.date = date;
    postFilter.value.page = 1;
    await loadPostQuery();
};

const loadReport = async (accountId = reportAccountId.value || snapshot.value.accounts[0]?.id) => {
    reportError.value = '';
    reportLoading.value = true;

    if (!accountId) {
        report.value = null;
        activeReportAudienceIndex.value = null;
        reportLoading.value = false;
        return;
    }

    reportAccountId.value = Number(accountId);

    try {
        report.value = await invoke('account_report', {
            request: {
                account_id: Number(accountId),
                period: reportPeriod.value,
            },
        });
        activeReportAudienceIndex.value = null;
    } catch (error) {
        report.value = null;
        activeReportAudienceIndex.value = null;
        reportError.value = String(error);
    } finally {
        reportLoading.value = false;
    }
};

const saveSettings = async () => {
    settingsSaving.value = true;
    settingsError.value = '';
    settingsSaved.value = false;

    try {
        const saved = await invoke('save_settings', {
            settings: {
                ...settingsDraft.value,
                time_format: Number(settingsDraft.value.time_format) || 12,
                week_starts_on: Number(settingsDraft.value.week_starts_on) || 0,
                operator_name: settingsDraft.value.operator_name || '',
                admin_email: settingsDraft.value.admin_email || '',
                desktop_notifications: Boolean(settingsDraft.value.desktop_notifications),
                local_ai_media_labs: Boolean(settingsDraft.value.local_ai_media_labs),
                default_accounts: settingsDraft.value.default_accounts.map((id) => Number(id)),
            },
        });

        settings.value = saved;
        settingsDraft.value = {
            ...saved,
            default_accounts: [...(saved.default_accounts || [])],
        };
        if (
            !editingPostUuid.value
            && !compactEditorText(draftBody.value)
            && !draftMediaIds.value.length
            && !draftExternalMedia.value.length
            && !draftTagIds.value.length
            && !draftScheduledAt.value
        ) {
            draftAccountIds.value = defaultDraftAccountIds(snapshot.value.accounts, saved);
        }
        settingsSaved.value = true;
        await load();
    } catch (error) {
        settingsError.value = String(error);
    } finally {
        settingsSaving.value = false;
    }
};

const refreshSystemLogs = async () => {
    systemLogRunning.value = true;
    systemLogError.value = '';

    try {
        systemLogs.value = await invoke('system_logs');
    } catch (error) {
        systemLogError.value = String(error);
    } finally {
        systemLogRunning.value = false;
    }
};

const systemStatusText = () => {
    const counts = health.value?.counts || {};
    const issues = health.value?.issues || [];

    return [
        '## Dust Wave Social Status',
        '',
        `View: ${activeViewDefinition.value.label}`,
        `Health: ${health.value?.status || 'unknown'}`,
        `Connected accounts: ${dashboard.value?.accounts?.authorized ?? 0}/${dashboard.value?.accounts?.total ?? 0}`,
        `Unauthorized accounts: ${counts.unauthorized_accounts ?? 0}`,
        `Scheduled posts: ${dashboard.value?.posts?.scheduled ?? 0}`,
        `Failed posts: ${counts.failed_posts ?? 0}`,
        `Pending jobs: ${counts.pending_jobs ?? 0}`,
        `Processing jobs: ${counts.processing_jobs ?? 0}`,
        `Failed jobs: ${counts.failed_jobs ?? 0}`,
        `Provider limits: ${counts.rate_limits ?? 0}`,
        '',
        '## Technical Details',
        systemTechnicalRows.value.map((row) => `**${row.label}**: ${row.value}`).join('\n'),
        '',
        '## Issues',
        issues.length ? issues.map((issue) => `- ${issue.severity}: ${issue.title} - ${issue.detail}`).join('\n') : '- None',
    ].join('\n');
};

const copySystemStatus = async () => {
    systemStatusCopied.value = false;
    appDataPathCopied.value = false;
    systemStatusCopyError.value = '';

    if (await writeClipboard(systemStatusText(), systemStatusCopyError)) {
        systemStatusCopied.value = true;
    }
};

const copyAppDataPath = async () => {
    appDataPathCopied.value = false;
    systemStatusCopied.value = false;
    systemStatusCopyError.value = '';

    try {
        const path = await invoke('app_data_directory');

        if (await writeClipboard(path, systemStatusCopyError)) {
            appDataPathCopied.value = true;
        }
    } catch (error) {
        systemStatusCopyError.value = String(error);
    }
};

const updaterErrorMessage = (error) => {
    const message = String(error || '');

    if (/pubkey|endpoint|updater|signature|URL/i.test(message)) {
        return 'Updater is not configured for this build. Build with a Tauri updater public key and GitHub Releases endpoint.';
    }

    return message || 'Software update check failed';
};

const checkSoftwareUpdate = async () => {
    softwareUpdateChecking.value = true;
    softwareUpdateError.value = '';
    softwareUpdateProgress.value = '';
    softwareUpdateStatus.value = 'Checking GitHub Releases for updates';

    try {
        const update = await checkForUpdate({ timeout: 15000 });
        softwareUpdateAvailable.value = update;

        if (update) {
            softwareUpdateStatus.value = `Version ${update.version} is available`;
            return;
        }

        softwareUpdateStatus.value = 'Dust Wave Social is up to date';
    } catch (error) {
        softwareUpdateAvailable.value = null;
        softwareUpdateError.value = updaterErrorMessage(error);
        softwareUpdateStatus.value = 'Update check unavailable';
    } finally {
        softwareUpdateChecking.value = false;
    }
};

const installSoftwareUpdate = async () => {
    const update = softwareUpdateAvailable.value;

    if (!update) {
        return;
    }

    let downloadedBytes = 0;
    let totalBytes = 0;
    softwareUpdateInstalling.value = true;
    softwareUpdateError.value = '';
    softwareUpdateProgress.value = '';
    softwareUpdateStatus.value = `Downloading version ${update.version}`;

    try {
        const onEvent = new Channel();
        onEvent.onmessage = (event) => {
            if (event.event === 'Started') {
                totalBytes = Number(event.data.contentLength) || 0;
                downloadedBytes = 0;
                softwareUpdateProgress.value = totalBytes
                    ? 'Downloading 0%'
                    : 'Downloading update';
            }

            if (event.event === 'Progress') {
                downloadedBytes += Number(event.data.chunkLength) || 0;
                const percentage = totalBytes
                    ? Math.min(100, Math.floor((downloadedBytes / totalBytes) * 100))
                    : 0;
                softwareUpdateProgress.value = totalBytes
                    ? `Downloading ${percentage}% · ${formatBytes(downloadedBytes)} of ${formatBytes(totalBytes)}`
                    : `Downloaded ${formatBytes(downloadedBytes)}`;
            }

            if (event.event === 'Finished') {
                softwareUpdateProgress.value = 'Verifying signed update';
                softwareUpdateStatus.value = `Verifying version ${update.version}`;
            }

            if (event.event === 'Installing') {
                softwareUpdateProgress.value = 'Installing update';
                softwareUpdateStatus.value = `Installing version ${update.version}`;
            }

            if (event.event === 'Restarting') {
                softwareUpdateProgress.value = 'Restarting Dust Wave Social';
                softwareUpdateStatus.value = `Version ${update.version} installed`;
            }
        };

        await invoke('install_software_update_and_restart', {
            expectedVersion: update.version,
            onEvent,
        });
    } catch (error) {
        softwareUpdateError.value = updaterErrorMessage(error);
        softwareUpdateStatus.value = 'Update install failed';
    } finally {
        softwareUpdateInstalling.value = false;
    }
};

const checkOrInstallSoftwareUpdate = async () => {
    if (softwareUpdateChecking.value || softwareUpdateInstalling.value) {
        return;
    }

    if (softwareUpdateAvailable.value) {
        await installSoftwareUpdate();
        return;
    }

    await checkSoftwareUpdate();
};

const exportSystemLog = async () => {
    systemLogRunning.value = true;
    systemLogError.value = '';
    systemLogExport.value = null;

    try {
        systemLogExport.value = await invoke('export_system_log');
        systemLogs.value = await invoke('system_logs');
    } catch (error) {
        systemLogError.value = String(error);
    } finally {
        systemLogRunning.value = false;
    }
};

const clearSystemLogs = async () => {
    if (!await requestConfirmation({
        title: 'Clear local system logs?',
        description: 'This removes the local redacted troubleshooting log history. Connected accounts, posts, media, and credentials are not changed.',
        confirmLabel: 'Clear logs',
        danger: true,
    })) {
        return;
    }

    systemLogRunning.value = true;
    systemLogError.value = '';
    systemLogExport.value = null;

    try {
        systemLogClearSummary.value = await invoke('clear_system_logs');
        systemLogs.value = await invoke('system_logs');
    } catch (error) {
        systemLogClearSummary.value = null;
        systemLogError.value = String(error);
    } finally {
        systemLogRunning.value = false;
    }
};

const createLocalBackup = async () => {
    backupRunning.value = true;
    backupError.value = '';
    backupSummary.value = null;

    try {
        backupSummary.value = await invoke('create_local_backup');
        restoreBackupPath.value = backupSummary.value.path;
        systemLogs.value = await invoke('system_logs');
    } catch (error) {
        backupError.value = String(error);
    } finally {
        backupRunning.value = false;
    }
};

const chooseRestoreBackupPath = async () => {
    restoreError.value = '';

    try {
        const directory = await pickDirectoryPath();

        if (directory) {
            restoreBackupPath.value = directory;
        }
    } catch (error) {
        restoreError.value = String(error);
    }
};

const restoreLocalBackup = async () => {
    if (!restoreBackupPath.value.trim()) {
        restoreError.value = 'Choose a backup folder first';
        return;
    }

    if (!await requestConfirmation({
        title: 'Restore this local backup?',
        description: 'Dust Wave Social will create a safety backup, replace current local data with the selected backup, and then reload the workspace. Keychain credentials must be reconnected separately.',
        confirmLabel: 'Restore backup',
        danger: true,
    })) {
        return;
    }

    restoreRunning.value = true;
    restoreError.value = '';
    restoreSummary.value = null;

    try {
        restoreSummary.value = await invoke('restore_local_backup', {
            backup: {
                backup_path: restoreBackupPath.value,
            },
        });
        window.localStorage.removeItem(DRAFT_STORAGE_KEY);
        resetDraftEditor();
        await load();
        systemLogs.value = await invoke('system_logs');
    } catch (error) {
        restoreError.value = String(error);
    } finally {
        restoreRunning.value = false;
    }
};

const clearResolvedSystemState = async () => {
    if (!await requestConfirmation({
        title: 'Clear resolved system state?',
        description: 'Completed and cancelled background work plus expired provider limits will be removed. Active and failed work will remain available for review.',
        confirmLabel: 'Clear resolved state',
        danger: true,
    })) {
        return;
    }

    maintenanceRunning.value = true;
    maintenanceError.value = '';

    try {
        maintenanceSummary.value = await invoke('clear_resolved_system_state', {
            now: new Date().toISOString(),
        });
        await load();
    } catch (error) {
        maintenanceSummary.value = null;
        maintenanceError.value = String(error);
    } finally {
        maintenanceRunning.value = false;
    }
};

const runDesktopMaintenance = async (options = {}) => {
    if (desktopMaintenanceRunning.value) {
        return;
    }

    const background = options?.background === true;
    desktopMaintenanceRunning.value = true;
    autoMaintenanceLastRun.value = new Date().toISOString();

    if (!background) {
        desktopMaintenanceError.value = '';
    }

    try {
        desktopMaintenanceSummary.value = await invoke('run_desktop_maintenance', {
            now: new Date().toISOString(),
        });

        const changed =
            desktopMaintenanceSummary.value?.resolved_state?.completed_jobs_deleted > 0 ||
            desktopMaintenanceSummary.value?.resolved_state?.cancelled_jobs_deleted > 0 ||
            desktopMaintenanceSummary.value?.resolved_state?.expired_rate_limits_cleared > 0 ||
            desktopMaintenanceSummary.value?.media?.deleted > 0;

        if (!background || changed) {
            await load();
        }
    } catch (error) {
        desktopMaintenanceError.value = String(error);
    } finally {
        desktopMaintenanceRunning.value = false;
    }
};

const recoverStaleProcessingJobs = async () => {
    staleRecoveryRunning.value = true;
    staleRecoveryError.value = '';

    try {
        const now = new Date();
        const staleBefore = new Date(now.getTime() - 15 * 60 * 1000);
        staleRecoverySummary.value = await invoke('recover_stale_processing_jobs', {
            now: now.toISOString(),
            staleBefore: staleBefore.toISOString(),
        });
        await load();
    } catch (error) {
        staleRecoverySummary.value = null;
        staleRecoveryError.value = String(error);
    } finally {
        staleRecoveryRunning.value = false;
    }
};

const retryFailedAccountImports = async () => {
    failedImportRetryRunning.value = true;
    failedImportRetryError.value = '';
    failedImportRetryJobs.value = [];

    try {
        failedImportRetryJobs.value = await invoke('retry_failed_account_import_jobs', {
            runAt: new Date().toISOString(),
        });
        await load();
    } catch (error) {
        failedImportRetryError.value = String(error);
    } finally {
        failedImportRetryRunning.value = false;
    }
};

const openAttentionNotice = async (notice) => {
    activeView.value = notice.view;

    if (notice.connectionTab) {
        activeConnectionTab.value = notice.connectionTab;
    }

    if (notice.status) {
        activePostsMode.value = 'library';
        postFilter.value.status = notice.status;
        postFilter.value.page = 1;
        await loadPostQuery();
    }
};

const serviceDefinitionByName = (serviceName) => {
    return serviceDefinitions.find((service) => service.id === serviceName) || null;
};

const serviceRecordByName = (serviceName) => {
    return snapshot.value.services.find((service) => service.name === serviceName) || null;
};

const serviceConfigurationValue = (serviceName, fieldName) => {
    return String(serviceConfigurationDrafts.value[serviceName]?.[fieldName] || '').trim();
};

const normalizedServiceBaseUrl = (serviceName, fieldName) => {
    const value = serviceConfigurationValue(serviceName, fieldName);

    if (!value) {
        return '';
    }

    try {
        const url = new URL(value);

        if (url.protocol !== 'https:') {
            return '';
        }

        url.search = '';
        url.hash = '';
        url.pathname = url.pathname.replace(/\/+$/, '') || '/';

        return url.toString().replace(/\/$/, '');
    } catch {
        return '';
    }
};

const appendServicePath = (baseUrl, path) => {
    const url = new URL(baseUrl);
    const basePath = url.pathname.replace(/\/+$/, '');

    url.pathname = `${basePath}/${path.replace(/^\/+/, '')}`;

    return url.toString();
};

const tiktokBrokerBaseUrl = () => normalizedServiceBaseUrl('tiktok', 'broker_base_url');
const mediaStagingBaseUrl = () => normalizedServiceBaseUrl('media_staging', 'base_url');
const tiktokBrokerAuthUrl = () => {
    const baseUrl = tiktokBrokerBaseUrl();

    return baseUrl ? appendServicePath(baseUrl, '/api/tiktok/oauth/start') : '';
};
const tiktokBrokerCallbackUrl = () => {
    const baseUrl = tiktokBrokerBaseUrl();

    return baseUrl ? appendServicePath(baseUrl, '/api/tiktok/oauth/callback') : '';
};

const serviceStatusByName = (serviceName) => {
    return credentialStatuses.value.find((status) => status.service === serviceName) || null;
};

const serviceReady = (serviceName) => {
    const status = serviceStatusByName(serviceName);

    if (serviceName === 'tiktok') {
        return Boolean(status?.configured && serviceActiveValue(serviceName) && tiktokBrokerBaseUrl());
    }

    if (serviceName === 'media_staging') {
        return Boolean(status?.configured && serviceActiveValue(serviceName) && mediaStagingBaseUrl());
    }

    return Boolean(status?.configured && serviceActiveValue(serviceName));
};

const showServiceTab = (serviceName) => {
    closeAddAccountModal();
    activeServiceTab.value = serviceName;
    activeConnectionTab.value = 'services';
    activeView.value = 'connections';
};

const selectServiceTab = (serviceName) => {
    activeServiceTab.value = serviceName;
    serviceError.value = '';
    serviceSaveMessage.value = '';
};

const clearServiceFeedback = () => {
    serviceError.value = '';
    serviceSaveMessage.value = '';
};

const serviceConfigurationPayload = (serviceName) => {
    return {
        ...(serviceConfigurationDrafts.value[serviceName] || {}),
    };
};

const serviceActiveValue = (serviceName) => {
    const status = serviceStatusByName(serviceName);
    const record = serviceRecordByName(serviceName);

    return status?.active ?? record?.active ?? false;
};

const serviceStatusText = (serviceName) => {
    const status = serviceStatusByName(serviceName);
    const active = serviceActiveValue(serviceName);

    if (serviceReady(serviceName)) {
        return 'active and configured';
    }

    if (status?.configured && active) {
        return 'active but setup incomplete';
    }

    if (status?.configured) {
        return 'configured but inactive';
    }

    if (active) {
        return 'active but missing credentials';
    }

    return 'missing credentials';
};

const serviceTabStatusText = (serviceName) => {
    if (serviceReady(serviceName)) {
        return 'ready';
    }

    const status = serviceStatusByName(serviceName);

    if (status?.configured && serviceActiveValue(serviceName)) {
        return 'incomplete';
    }

    return status?.configured ? 'inactive' : 'missing';
};

const serviceCredentialInputHint = (fieldName) => {
    const field = activeServiceConfiguredFields.value.get(fieldName);

    if (field?.configured) {
        return 'Available — leave blank to keep current value';
    }

    if (field?.env_vars?.length) {
        return `Required · env fallback: ${field.env_vars.join(' or ')}`;
    }

    return 'Required';
};

const serviceCredentialStatusText = (service, credential) => {
    const status = serviceStatusByName(service.id);
    const field = status?.fields?.find((item) => item.field === credential.field);

    if (field?.configured) {
        return 'saved in keychain';
    }

    if (field?.env_vars?.length) {
        return `missing; env fallback: ${field.env_vars.join(' or ')}`;
    }

    return 'missing';
};

const clearSavedServiceCredentialDrafts = (serviceName, savedFields) => {
    if (!savedFields.length) {
        return;
    }

    const nextServiceDraft = {
        ...(serviceCredentialDrafts.value[serviceName] || {}),
    };

    savedFields.forEach((fieldName) => {
        nextServiceDraft[fieldName] = '';
    });

    serviceCredentialDrafts.value = {
        ...serviceCredentialDrafts.value,
        [serviceName]: nextServiceDraft,
    };
};

const saveServiceSettings = async (serviceName = activeServiceTab.value) => {
    const definition = serviceDefinitionByName(serviceName);
    const record = serviceRecordByName(serviceName);

    if (!definition) {
        serviceError.value = 'Unknown service';
        return;
    }

    const active = Boolean(serviceActiveDrafts.value[serviceName]);
    const previousActive = serviceActiveValue(serviceName);
    const statusFields = new Map(
        (serviceStatusByName(serviceName)?.fields || []).map((field) => [field.field, field]),
    );
    const missingCredentials = definition.credentials.filter((credential) => {
        const draftValue = serviceCredentialDrafts.value[serviceName]?.[credential.field] || '';

        return !draftValue.trim() && !statusFields.get(credential.field)?.configured;
    });

    if (active && missingCredentials.length) {
        serviceSaveMessage.value = '';
        serviceError.value = `Add ${missingCredentials.map((credential) => credential.label).join(' and ')} before activating ${definition.label}.`;
        return;
    }

    serviceSaving.value = true;
    serviceError.value = '';
    serviceSaveMessage.value = '';
    const savedFields = [];

    try {
        for (const credential of definition.credentials) {
            const value = serviceCredentialDrafts.value[serviceName]?.[credential.field] || '';

            if (!value.trim()) {
                continue;
            }

            await invoke('save_service_credential', {
                credential: {
                    service: serviceName,
                    field: credential.field,
                    value,
                },
            });
            savedFields.push(credential.field);
        }

        await invoke('save_service', {
            service: {
                name: serviceName,
                configuration_secret_ref: record?.configuration_secret_ref || definition.configurationSecretRef,
                configuration: serviceConfigurationPayload(serviceName),
                active,
            },
        });
        clearSavedServiceCredentialDrafts(serviceName, savedFields);
        await load();
        const credentialSummary = savedFields.length
            ? `${pluralize(savedFields.length, 'credential')} stored in Keychain.`
            : 'Existing credentials were left unchanged.';
        const readinessSummary = active
            ? `Service is active${serviceReady(serviceName) ? ' and ready' : ', but setup is incomplete'}.`
            : 'Service is inactive.';
        serviceSaveMessage.value = `${definition.label} settings saved. ${credentialSummary} ${readinessSummary}`;
    } catch (error) {
        clearSavedServiceCredentialDrafts(serviceName, savedFields);
        let stateRestoreWarning = '';

        if (savedFields.length) {
            try {
                await invoke('save_service', {
                    service: {
                        name: serviceName,
                        configuration_secret_ref: record?.configuration_secret_ref || definition.configurationSecretRef,
                        configuration: null,
                        active: previousActive,
                    },
                });
            } catch {
                stateRestoreWarning = ' The previous Active state could not be restored; review it before continuing.';
            }
        }

        const partialSaveWarning = savedFields.length
            ? ` ${pluralize(savedFields.length, 'credential')} saved before the failure remain in Keychain.`
            : '';
        serviceError.value = `Could not finish saving ${definition.label} settings.${partialSaveWarning}${stateRestoreWarning} ${String(error)}`;

        if (savedFields.length) {
            await load();
        }
    } finally {
        serviceSaving.value = false;
    }
};

const openServiceUrl = async (url) => {
    clearServiceFeedback();

    if (url) {
        await openOAuthUrl(url, serviceError);
    }
};

const openTikTokBrokerAuthWithTarget = async (errorTarget) => {
    errorTarget.value = '';

    const authUrl = tiktokBrokerAuthUrl();

    if (!authUrl) {
        errorTarget.value = 'Save an HTTPS TikTok Broker URL before starting TikTok authorization';
        return;
    }

    await openOAuthUrl(authUrl, errorTarget);
};

const openTikTokBrokerAuth = async () => {
    await openTikTokBrokerAuthWithTarget(tiktokConnectionError);
};

const openTikTokBrokerAuthFromServices = async () => {
    clearServiceFeedback();
    await openTikTokBrokerAuthWithTarget(serviceError);
};

const serviceSetupFieldValue = (service, field) => {
    if (service.id === 'tiktok' && field.key === 'redirect') {
        return tiktokBrokerCallbackUrl() || field.value;
    }

    return field.value;
};

const serviceSetupText = (service) => {
    const lines = [
        `${service.label} setup`,
        `Status: ${serviceStatusText(service.id)}`,
        `Create app: ${service.setupUrl}`,
    ];

    if (service.docsUrl) {
        lines.push(`Docs: ${service.docsUrl}`);
    }

    for (const field of service.setupFields || []) {
        lines.push(`${field.label}: ${serviceSetupFieldValue(service, field)}`);
    }

    if (service.credentials?.length) {
        lines.push('Credential fields:');

        for (const credential of service.credentials) {
            lines.push(`- ${credential.label}: ${serviceCredentialStatusText(service, credential)}`);
        }
    }

    if (service.configuration?.length) {
        lines.push(`Configuration: ${service.configuration.map((field) => `${field.label}=${serviceConfigurationDrafts.value[service.id]?.[field.field] || field.defaultValue}`).join(', ')}`);
    }

    return lines.join('\n');
};

const copyServiceSetup = async (service) => {
    clearServiceFeedback();

    if (await writeClipboard(serviceSetupText(service), serviceError)) {
        serviceSetupCopied.value = `${service.id}:setup`;
    }
};

const copyServiceSetupField = async (service, field) => {
    clearServiceFeedback();

    if (await writeClipboard(serviceSetupFieldValue(service, field), serviceError)) {
        serviceSetupCopied.value = `${service.id}:${field.key}`;
    }
};

const providerSetupBundleText = (onlyMissing = false) => {
    const services = serviceDefinitions.filter((service) => !onlyMissing || !serviceReady(service.id));
    const header = [
        'Dust Wave Social provider setup packet',
        `Generated: ${new Date().toLocaleString()}`,
        `Scope: ${onlyMissing ? 'missing or inactive services' : 'all services'}`,
        '',
        'Use this packet to create or update provider developer apps, then save the returned credentials in Connections > Provider setup. Secret values are intentionally not included.',
        '',
        'Next actions',
        '1. Open each Create App URL and paste the callback URLs, scopes, and setup values below.',
        '2. Enter each provider client ID/API key, secret, and configuration in the matching Provider setup form.',
        '3. Select Active, use the single Save Settings action, then use Connected accounts to start OAuth for each Dust Wave account.',
    ];

    if (!services.length) {
        return [
            ...header,
            '',
            'All configured services are already active. Use Connected accounts to continue onboarding.',
        ].join('\n');
    }

    return [
        header.join('\n'),
        ...services.map((service) => serviceSetupText(service)),
    ].join('\n\n---\n\n');
};

const copyProviderSetupBundle = async (onlyMissing = false) => {
    clearServiceFeedback();

    if (await writeClipboard(providerSetupBundleText(onlyMissing), serviceError)) {
        serviceSetupCopied.value = onlyMissing ? 'bundle:missing' : 'bundle:all';
    }
};

const startTwitterOAuth = async () => {
    twitterOAuthSaving.value = true;
    twitterOAuthError.value = '';

    try {
        twitterOAuthStart.value = await invoke('start_twitter_oauth', {
            request: {
                redirect_uri: twitterOAuthDraft.value.redirect_uri || null,
                scopes: [],
            },
        });
        twitterOAuthDraft.value.redirect_uri = twitterOAuthStart.value.redirect_uri;
        twitterOAuthDraft.value.code_verifier = twitterOAuthStart.value.code_verifier;
        await openOAuthUrl(twitterOAuthStart.value.auth_url, twitterOAuthError);
    } catch (error) {
        twitterOAuthStart.value = null;
        twitterOAuthError.value = String(error);
    } finally {
        twitterOAuthSaving.value = false;
    }
};

const connectTwitterAccount = async () => {
    if (!twitterOAuthDraft.value.code.trim()) {
        twitterOAuthError.value = 'Authorization code is required';
        return;
    }

    if (!twitterOAuthDraft.value.code_verifier.trim()) {
        twitterOAuthError.value = 'Code verifier is required';
        return;
    }

    twitterOAuthSaving.value = true;
    twitterOAuthError.value = '';

    try {
        twitterOAuthConnection.value = await invoke('connect_twitter_account', {
            request: {
                code: twitterOAuthDraft.value.code,
                code_verifier: twitterOAuthDraft.value.code_verifier,
                redirect_uri: twitterOAuthDraft.value.redirect_uri || null,
            },
        });
        twitterOAuthDraft.value.code = '';
        await load();
    } catch (error) {
        twitterOAuthConnection.value = null;
        twitterOAuthError.value = String(error);
    } finally {
        twitterOAuthSaving.value = false;
    }
};

const startFacebookOAuth = async () => {
    facebookOAuthSaving.value = true;
    facebookOAuthError.value = '';

    try {
        facebookOAuthStart.value = await invoke('start_facebook_oauth', {
            request: {
                redirect_uri: facebookOAuthDraft.value.redirect_uri || null,
                scopes: [],
            },
        });
        facebookOAuthDraft.value.redirect_uri = facebookOAuthStart.value.redirect_uri;
        await openOAuthUrl(facebookOAuthStart.value.auth_url, facebookOAuthError);
    } catch (error) {
        facebookOAuthStart.value = null;
        facebookOAuthError.value = String(error);
    } finally {
        facebookOAuthSaving.value = false;
    }
};

const exchangeFacebookOAuth = async () => {
    if (!facebookOAuthDraft.value.code.trim()) {
        facebookOAuthError.value = 'Authorization code is required';
        return;
    }

    facebookOAuthSaving.value = true;
    facebookOAuthError.value = '';

    try {
        facebookUserConnection.value = await invoke('exchange_facebook_oauth', {
            request: {
                code: facebookOAuthDraft.value.code,
                redirect_uri: facebookOAuthDraft.value.redirect_uri || null,
            },
        });
        facebookOAuthDraft.value.code = '';
        facebookOAuthDraft.value.selected_pages = facebookUserConnection.value.pages.map((page) => page.id);
        facebookOAuthDraft.value.selected_instagram_accounts = (facebookUserConnection.value.instagram_accounts || []).map((account) => account.id);
    } catch (error) {
        facebookUserConnection.value = null;
        facebookOAuthError.value = String(error);
    } finally {
        facebookOAuthSaving.value = false;
    }
};

const connectFacebookPages = async () => {
    if (!facebookUserConnection.value) {
        facebookOAuthError.value = 'Exchange Facebook OAuth before saving pages';
        return;
    }

    if (!facebookOAuthDraft.value.selected_pages.length) {
        facebookOAuthError.value = 'Select at least one Facebook Page';
        return;
    }

    facebookOAuthSaving.value = true;
    facebookOAuthError.value = '';

    try {
        facebookPageConnection.value = await invoke('connect_facebook_pages', {
            request: {
                user_id: facebookUserConnection.value.user_id,
                page_ids: facebookOAuthDraft.value.selected_pages,
            },
        });
        await load();
    } catch (error) {
        facebookPageConnection.value = null;
        facebookOAuthError.value = String(error);
    } finally {
        facebookOAuthSaving.value = false;
    }
};

const connectInstagramAccounts = async () => {
    if (!facebookUserConnection.value) {
        facebookOAuthError.value = 'Exchange Facebook OAuth before saving Instagram accounts';
        return;
    }

    if (!facebookOAuthDraft.value.selected_instagram_accounts.length) {
        facebookOAuthError.value = 'Select at least one Instagram account';
        return;
    }

    facebookOAuthSaving.value = true;
    facebookOAuthError.value = '';

    try {
        instagramAccountConnection.value = await invoke('connect_instagram_accounts', {
            request: {
                user_id: facebookUserConnection.value.user_id,
                instagram_ids: facebookOAuthDraft.value.selected_instagram_accounts,
            },
        });
        await load();
    } catch (error) {
        instagramAccountConnection.value = null;
        facebookOAuthError.value = String(error);
    } finally {
        facebookOAuthSaving.value = false;
    }
};

const selectAllFacebookPages = () => {
    facebookOAuthDraft.value.selected_pages = (facebookUserConnection.value?.pages || []).map((page) => page.id);
};

const clearFacebookPages = () => {
    facebookOAuthDraft.value.selected_pages = [];
};

const facebookPageIsSelected = (page) => {
    return facebookOAuthDraft.value.selected_pages.includes(page.id);
};

const selectAllInstagramAccounts = () => {
    facebookOAuthDraft.value.selected_instagram_accounts = (facebookUserConnection.value?.instagram_accounts || []).map((account) => account.id);
};

const clearInstagramAccounts = () => {
    facebookOAuthDraft.value.selected_instagram_accounts = [];
};

const instagramAccountIsSelected = (account) => {
    return facebookOAuthDraft.value.selected_instagram_accounts.includes(account.id);
};

const registerMastodonApp = async () => {
    if (!mastodonAppDraft.value.server.trim()) {
        mastodonAppError.value = 'Mastodon server is required';
        return;
    }

    mastodonAppSaving.value = true;
    mastodonAppError.value = '';

    try {
        mastodonAppRegistration.value = await invoke('register_mastodon_app', {
            request: {
                ...mastodonAppDraft.value,
                website: mastodonAppDraft.value.website || null,
            },
        });
        mastodonOAuthDraft.value.server = mastodonAppRegistration.value.server;
        await openOAuthUrl(mastodonAppRegistration.value.auth_url, mastodonAppError);
        await load();
    } catch (error) {
        mastodonAppError.value = String(error);
    } finally {
        mastodonAppSaving.value = false;
    }
};

const connectMastodonAccount = async () => {
    if (!mastodonOAuthDraft.value.server.trim()) {
        mastodonOAuthError.value = 'Mastodon server is required';
        return;
    }

    if (!mastodonOAuthDraft.value.code.trim()) {
        mastodonOAuthError.value = 'Authorization code is required';
        return;
    }

    mastodonOAuthSaving.value = true;
    mastodonOAuthError.value = '';

    try {
        mastodonOAuthConnection.value = await invoke('connect_mastodon_account', {
            request: {
                ...mastodonOAuthDraft.value,
                redirect_uri: mastodonOAuthDraft.value.redirect_uri || null,
            },
        });
        mastodonOAuthDraft.value.code = '';
        await load();
    } catch (error) {
        mastodonOAuthConnection.value = null;
        mastodonOAuthError.value = String(error);
    } finally {
        mastodonOAuthSaving.value = false;
    }
};

const connectTikTokAccount = async () => {
    if (!tiktokConnectionDraft.value.provider_id.trim()) {
        tiktokConnectionError.value = 'TikTok user ID is required';
        return;
    }

    if (!tiktokConnectionDraft.value.name.trim()) {
        tiktokConnectionError.value = 'TikTok display name is required';
        return;
    }

    if (!tiktokConnectionDraft.value.broker_connection.trim()) {
        tiktokConnectionError.value = 'Broker connection credential is required';
        return;
    }

    tiktokConnectionSaving.value = true;
    tiktokConnectionError.value = '';

    try {
        tiktokConnection.value = await invoke('connect_tiktok_account', {
            request: {
                provider_id: tiktokConnectionDraft.value.provider_id,
                name: tiktokConnectionDraft.value.name,
                username: tiktokConnectionDraft.value.username || null,
                avatar_path: tiktokConnectionDraft.value.avatar_path || null,
                broker_connection: tiktokConnectionDraft.value.broker_connection,
                scopes: tiktokConnectionDraft.value.scopes
                    .split(',')
                    .map((scope) => scope.trim())
                    .filter(Boolean),
            },
        });
        tiktokConnectionDraft.value.broker_connection = '';
        await load();
    } catch (error) {
        tiktokConnection.value = null;
        tiktokConnectionError.value = String(error);
    } finally {
        tiktokConnectionSaving.value = false;
    }
};

const createTag = async () => {
    tagSaving.value = true;
    tagError.value = '';

    try {
        await invoke('create_tag', {
            tag: tagDraft.value,
        });

        tagDraft.value.name = '';
        await load();
    } catch (error) {
        tagError.value = String(error);
    } finally {
        tagSaving.value = false;
    }
};

const editTag = (tag) => {
    editingTagUuid.value = tag.uuid;
    tagEditDraft.value = {
        name: tag.name,
        hex_color: `#${tag.hex_color}`,
    };
};

const cancelEditTag = () => {
    editingTagUuid.value = '';
    tagEditDraft.value = {
        name: '',
        hex_color: '#2f80ed',
    };
};

const updateTag = async (uuid) => {
    tagSaving.value = true;
    tagError.value = '';

    try {
        await invoke('update_tag', {
            uuid,
            tag: tagEditDraft.value,
        });
        cancelEditTag();
        await load();
    } catch (error) {
        tagError.value = String(error);
    } finally {
        tagSaving.value = false;
    }
};

const deleteTag = async (uuid) => {
    if (!await requestConfirmation({
        title: 'Delete this label?',
        description: 'Posts will remain, but this label will be removed from every post that currently uses it.',
        confirmLabel: 'Delete label',
        danger: true,
    })) {
        return;
    }

    tagSaving.value = true;
    tagError.value = '';

    try {
        await invoke('delete_tag', { uuid });
        await load();
    } catch (error) {
        tagError.value = String(error);
    } finally {
        tagSaving.value = false;
    }
};

const deleteAccount = async (uuid) => {
    if (!await requestConfirmation({
        title: 'Disconnect this account?',
        description: 'Existing post history will remain, but publishing and imports will stop until the account is reconnected.',
        confirmLabel: 'Disconnect account',
        danger: true,
    })) {
        return;
    }

    accountSaving.value = true;
    accountError.value = '';

    try {
        await invoke('delete_account', { uuid });
        draftAccountIds.value = draftAccountIds.value.filter((id) => {
            return snapshot.value.accounts.some((account) => account.id === id && account.uuid !== uuid);
        });
        await load();
    } catch (error) {
        accountError.value = String(error);
    } finally {
        accountSaving.value = false;
    }
};

const accountProviderOperations = {
    mastodon: {
        refreshCommand: 'refresh_mastodon_account',
        importCommand: 'import_mastodon_account_data',
        importSummary: mastodonImportSummary,
    },
    twitter: {
        refreshCommand: 'refresh_twitter_account',
        importCommand: 'import_twitter_account_data',
        importSummary: twitterImportSummary,
    },
    facebook_page: {
        refreshCommand: 'refresh_facebook_page_account',
        importCommand: 'import_facebook_page_data',
        importSummary: facebookImportSummary,
    },
    instagram: {
        refreshCommand: 'refresh_instagram_account',
        importCommand: 'import_instagram_account_data',
        importSummary: instagramImportSummary,
    },
    tiktok: {
        importCommand: 'import_tiktok_account_data',
        importSummary: tiktokImportSummary,
    },
};

const accountProviderOperation = (account) => accountProviderOperations[account.provider] || null;
const accountCanRefresh = (account) => Boolean(accountProviderOperation(account)?.refreshCommand);

const refreshAccount = async (account) => {
    const operation = accountProviderOperation(account);

    if (!operation?.refreshCommand) {
        return;
    }

    accountRefreshingUuid.value = account.uuid;
    accountError.value = '';

    try {
        await invoke(operation.refreshCommand, { uuid: account.uuid });
        await load();
    } catch (error) {
        accountError.value = String(error);
        await load();
    } finally {
        accountRefreshingUuid.value = '';
    }
};

const importAccountData = async (account) => {
    const operation = accountProviderOperation(account);

    if (!operation?.importCommand) {
        return;
    }

    accountImportingUuid.value = account.uuid;
    accountError.value = '';

    try {
        operation.importSummary.value = await invoke(operation.importCommand, { uuid: account.uuid });
        await load();
    } catch (error) {
        accountError.value = String(error);
        await load();
    } finally {
        accountImportingUuid.value = '';
    }
};

const queueAccountImport = async (uuid) => {
    accountQueuingUuid.value = uuid;
    queuedImportError.value = '';
    queuedImportJob.value = null;
    queuedImportBatch.value = null;

    try {
        queuedImportJob.value = await invoke('queue_account_import', { uuid });
        await load();
    } catch (error) {
        queuedImportError.value = String(error);
        await load();
    } finally {
        accountQueuingUuid.value = '';
    }
};

const queueAllAccountImports = async () => {
    queueAllImportsRunning.value = true;
    queuedImportError.value = '';
    queuedImportJob.value = null;
    queuedImportBatch.value = null;

    try {
        queuedImportBatch.value = await invoke('queue_all_account_imports');
        await load();
    } catch (error) {
        queuedImportError.value = String(error);
        await load();
    } finally {
        queueAllImportsRunning.value = false;
    }
};

const droppedMediaPath = (event) => {
    const file = event.dataTransfer?.files?.[0];

    if (file?.path) {
        return {
            path: file.path,
            name: file.name || '',
        };
    }

    const uri = event.dataTransfer
        ?.getData('text/uri-list')
        ?.split('\n')
        ?.map((value) => value.trim())
        ?.find((value) => value.startsWith('file://'));

    if (uri) {
        try {
            const path = decodeURIComponent(new URL(uri).pathname);

            return {
                path,
                name: path.split('/').pop() || '',
            };
        } catch (_error) {
            return null;
        }
    }

    return null;
};

const handleMediaDrop = (event) => {
    mediaDropActive.value = false;
    mediaError.value = '';
    const dropped = droppedMediaPath(event);

    if (!dropped?.path) {
        mediaError.value = 'Dropped file path is unavailable. Use Choose to select the file.';
        return;
    }

    mediaImport.value.source_path = dropped.path;

    if (!mediaImport.value.name && dropped.name) {
        mediaImport.value.name = dropped.name;
    }
};

const importMediaFile = async () => {
    if (!mediaImport.value.source_path.trim()) {
        mediaError.value = 'Local file path is required';
        return;
    }

    mediaSaving.value = true;
    mediaError.value = '';
    mediaImportResults.value = [];
    mediaProgress.value = 'Importing media';

    try {
        const media = await invoke('import_media_file', {
            media: {
                source_path: mediaImport.value.source_path,
                name: mediaImport.value.name || null,
            },
        });

        mediaImportResults.value = [{
            path: mediaImport.value.source_path,
            name: media.name || fileNameFromPath(mediaImport.value.source_path),
            status: 'imported',
            detail: media.mime_type,
        }];
        mediaImport.value.source_path = '';
        mediaImport.value.name = '';
        await load();
    } catch (error) {
        mediaError.value = String(error);
        mediaImportResults.value = [{
            path: mediaImport.value.source_path,
            name: fileNameFromPath(mediaImport.value.source_path),
            status: 'failed',
            detail: String(error),
        }];
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const downloadExternalMedia = async () => {
    if (!mediaDownload.value.url.trim()) {
        mediaError.value = 'Media URL is required';
        return;
    }

    mediaSaving.value = true;
    mediaError.value = '';
    mediaProgress.value = 'Downloading media';

    try {
        await invoke('download_external_media', {
            media: {
                url: mediaDownload.value.url,
                name: mediaDownload.value.name || null,
                source: mediaDownload.value.source || 'url',
                download_data: {},
            },
        });

        mediaDownload.value.url = '';
        mediaDownload.value.name = '';
        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const downloadExternalMediaItem = async (item) => {
    if (!canDownloadExternalMediaItem(item)) {
        mediaError.value = 'Klipy GIFs cannot be saved to the reusable media library.';
        return;
    }

    mediaSaving.value = true;
    mediaError.value = '';
    mediaProgress.value = `Downloading ${item.name || 'media'}`;

    try {
        await invoke('download_external_media', {
            media: {
                url: item.url,
                name: item.name || null,
                source: externalMediaResults.value?.source || externalMediaSearch.value.source,
                download_data: item.download_data ?? {},
            },
        });

        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const createPostFromMediaIds = async (mediaIds, externalMedia = []) => {
    await invoke('create_draft_post', {
        post: {
            accounts: [],
            tags: [],
            scheduled_at: null,
            versions: [
                {
                    account_id: 0,
                    is_original: true,
                    content: [
                        {
                            body: '',
                            media: mediaIds.map((id) => Number(id)),
                            external_media: externalMedia,
                        },
                    ],
                },
            ],
        },
    });
};

const createPostFromSelectedMedia = async () => {
    mediaSaving.value = true;
    mediaError.value = '';
    mediaProgress.value = 'Creating post from media';

    try {
        const mediaIds = selectedMediaIds.value.map((id) => Number(id));

        let externalMedia = [];

        if (activeMediaTab.value !== 'uploads' && selectedExternalMediaIncludesKlipy.value) {
            externalMedia = selectedExternalMediaItems.value.map(externalMediaReference);
        } else if (activeMediaTab.value !== 'uploads') {
            for (const item of selectedExternalMediaItems.value) {
                const saved = await invoke('download_external_media', {
                    media: {
                        url: item.url,
                        name: item.name || null,
                        source: externalMediaResults.value?.source || externalMediaSearch.value.source,
                        download_data: item.download_data ?? {},
                    },
                });
                mediaIds.push(saved.id);
            }
        }

        if (!mediaIds.length && !externalMedia.length) {
            mediaError.value = 'Select at least one media item';
            return;
        }

        await createPostFromMediaIds(mediaIds, externalMedia);
        selectedMediaIds.value = [];
        selectedExternalMediaIds.value = [];
        activeView.value = 'posts';
        activePostsMode.value = 'compose';
        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const deleteMedia = async (uuid) => {
    if (!await requestConfirmation({
        title: 'Delete this media item?',
        description: 'The app-owned copy will be removed permanently. The original file outside Dust Wave Social will not be changed.',
        confirmLabel: 'Delete media',
        danger: true,
    })) {
        return;
    }

    mediaSaving.value = true;
    mediaError.value = '';
    mediaProgress.value = 'Deleting media';

    try {
        await invoke('delete_media', { uuid });
        draftMediaIds.value = draftMediaIds.value.filter((id) => {
            return snapshot.value.media.some((item) => item.id === id && item.uuid !== uuid);
        });
        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const deleteSelectedMedia = async () => {
    if (activeMediaTab.value !== 'uploads') {
        mediaError.value = 'Only uploaded media can be deleted';
        return;
    }

    const selected = new Set(selectedMediaIds.value.map((id) => Number(id)));
    const items = mediaLibrary.value.filter((item) => selected.has(Number(item.id)));

    if (!items.length) {
        mediaError.value = 'Select at least one media item';
        return;
    }

    if (!await requestConfirmation({
        title: `Delete ${items.length} selected media item(s)?`,
        description: 'The app-owned copies will be removed permanently. Original files outside Dust Wave Social will not be changed.',
        confirmLabel: 'Delete selected media',
        danger: true,
    })) {
        return;
    }

    mediaSaving.value = true;
    mediaError.value = '';

    try {
        for (const [index, item] of items.entries()) {
            mediaProgress.value = `Deleting ${index + 1} of ${items.length}`;
            await invoke('delete_media', { uuid: item.uuid });
        }

        const deletedIds = new Set(items.map((item) => Number(item.id)));
        selectedMediaIds.value = [];
        draftMediaIds.value = draftMediaIds.value.filter((id) => !deletedIds.has(Number(id)));
        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const cleanupMediaFiles = async () => {
    mediaSaving.value = true;
    mediaError.value = '';
    mediaCleanup.value = null;
    mediaProgress.value = 'Cleaning orphaned media files';

    try {
        mediaCleanup.value = await invoke('cleanup_orphaned_media_files');
        await load();
    } catch (error) {
        mediaError.value = String(error);
    } finally {
        mediaProgress.value = '';
        mediaSaving.value = false;
    }
};

const resetDraftEditor = () => {
    editingPostUuid.value = '';
    contextualEditOrigin.value = '';
    draftBody.value = '';
    draftAccountIds.value = defaultDraftAccountIds();
    draftAccountBodies.value = {};
    activeDraftVersion.value = 0;
    draftMediaIds.value = [];
    draftExternalMedia.value = [];
    draftTagIds.value = [];
    draftScheduledAt.value = '';
    window.localStorage.removeItem(DRAFT_STORAGE_KEY);
};

const createPostFromCalendarDate = (date) => {
    resetDraftEditor();
    draftScheduledAt.value = `${date}T09:00`;
    activeView.value = 'posts';
    activePostsMode.value = 'compose';
};

const createPostFromCalendarSlot = (date, hour) => {
    resetDraftEditor();
    draftScheduledAt.value = `${date}T${String(hour).padStart(2, '0')}:00`;
    activeView.value = 'posts';
    activePostsMode.value = 'compose';
};

const toDateTimeLocal = (value) => {
    if (!value) {
        return '';
    }

    const date = new Date(value);

    if (Number.isNaN(date.getTime())) {
        return '';
    }

    date.setMinutes(date.getMinutes() - date.getTimezoneOffset());

    return date.toISOString().slice(0, 16);
};

const postPayload = (body) => {
    const versions = [
        {
            account_id: 0,
            is_original: true,
            content: [
                {
                    body,
                    media: draftMediaIds.value,
                    external_media: draftExternalMedia.value,
                },
            ],
        },
    ];

    for (const accountId of draftAccountIds.value) {
        const accountBody = normalizeEditorContent(draftAccountBodies.value[accountId] || '');

        if (!compactEditorText(accountBody)) {
            continue;
        }

        versions.push({
            account_id: Number(accountId),
            is_original: false,
            content: [
                {
                    body: accountBody,
                    media: draftMediaIds.value,
                    external_media: draftExternalMedia.value,
                },
            ],
        });
    }

    return {
        accounts: draftAccountIds.value,
        tags: draftTagIds.value,
        scheduled_at: draftScheduledAt.value ? draftScheduleDate.value?.toISOString() || null : null,
        versions,
    };
};

const composerHasDraftContent = () => {
    return Boolean(
        editingPostUuid.value
        || compactEditorText(draftBody.value)
        || Object.values(draftAccountBodies.value).some((value) => compactEditorText(value))
        || draftMediaIds.value.length
        || draftExternalMedia.value.length
        || draftTagIds.value.length
        || draftScheduledAt.value,
    );
};

const persistComposerDraft = () => {
    if (!composerHasDraftContent()) {
        window.localStorage.removeItem(DRAFT_STORAGE_KEY);
        return;
    }

    window.localStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify({
        editingPostUuid: editingPostUuid.value,
        body: draftBody.value,
        accounts: draftAccountIds.value,
        accountBodies: draftAccountBodies.value,
        activeVersion: activeDraftVersion.value,
        media: draftMediaIds.value,
        externalMedia: draftExternalMedia.value,
        tags: draftTagIds.value,
        scheduledAt: draftScheduledAt.value,
    }));
};

const restoreComposerDraft = () => {
    const saved = window.localStorage.getItem(DRAFT_STORAGE_KEY);

    if (!saved) {
        return;
    }

    try {
        const draft = JSON.parse(saved);

        editingPostUuid.value = draft.editingPostUuid || '';
        draftBody.value = draft.body || '';
        draftAccountIds.value = Array.isArray(draft.accounts) ? draft.accounts : [];
        draftAccountBodies.value = draft.accountBodies && typeof draft.accountBodies === 'object' ? draft.accountBodies : {};
        activeDraftVersion.value = Number(draft.activeVersion) || 0;
        draftMediaIds.value = Array.isArray(draft.media) ? draft.media : [];
        draftExternalMedia.value = Array.isArray(draft.externalMedia) ? draft.externalMedia : [];
        draftTagIds.value = Array.isArray(draft.tags) ? draft.tags : [];
        draftScheduledAt.value = draft.scheduledAt || '';
    } catch (_error) {
        window.localStorage.removeItem(DRAFT_STORAGE_KEY);
    }
};

const persistDraftPost = async ({ resetAfterSave = true } = {}) => {
    const body = normalizeEditorContent(draftBody.value);

    if (!compactEditorText(body) && !draftMediaIds.value.length && !draftExternalMedia.value.length) {
        draftError.value = 'Draft needs text or media';
        return null;
    }

    if (draftScheduledAt.value && !draftScheduleDate.value) {
        draftError.value = 'Enter a valid schedule time';
        return null;
    }

    draftSaving.value = true;
    draftError.value = '';

    try {
        let savedPost;

        if (editingPostUuid.value) {
            savedPost = await invoke('update_post', {
                uuid: editingPostUuid.value,
                post: postPayload(body),
            });
        } else {
            savedPost = await invoke('create_draft_post', {
                post: postPayload(body),
            });
        }

        if (resetAfterSave) {
            resetDraftEditor();
        } else if (savedPost?.uuid) {
            editingPostUuid.value = savedPost.uuid;
        }

        await load();
        return savedPost;
    } catch (error) {
        draftError.value = String(error);
        return null;
    } finally {
        draftSaving.value = false;
    }
};

const saveDraftPost = async () => {
    await persistDraftPost();
};

const clearDraftScheduleTime = () => {
    draftScheduledAt.value = '';
};

const scheduleCurrentDraft = async ({ postNow = false } = {}) => {
    scheduleError.value = '';

    if (!canSaveDraft.value || !draftAccountIds.value.length) {
        scheduleError.value = 'Select at least one account and write post copy before scheduling';
        return;
    }

    if (draftScheduledAt.value && !draftScheduleDate.value) {
        scheduleError.value = 'Enter a valid schedule time';
        return;
    }

    if (!postNow && (!draftScheduledAt.value || !draftScheduleDate.value)) {
        scheduleError.value = 'Schedule time is required';
        return;
    }

    const savedPost = await persistDraftPost({ resetAfterSave: false });

    if (!savedPost?.uuid && !editingPostUuid.value) {
        return;
    }

    scheduleSaving.value = true;

    try {
        await invoke('schedule_post', {
            uuid: savedPost?.uuid || editingPostUuid.value,
            schedule: {
                scheduled_at: postNow ? new Date().toISOString() : draftScheduleDate.value.toISOString(),
            },
        });

        postNowConfirmationOpen.value = false;
        resetDraftEditor();
        await load();
    } catch (error) {
        scheduleError.value = String(error);
    } finally {
        scheduleSaving.value = false;
    }
};

const openPostDetail = async (post) => {
    selectedPostSummary.value = post;
    selectedPostDetail.value = null;
    postDetailLoading.value = true;
    postDetailError.value = '';

    try {
        selectedPostDetail.value = await invoke('post_detail', { uuid: post.uuid });
    } catch (error) {
        postDetailError.value = String(error);
    } finally {
        postDetailLoading.value = false;
    }
};

const closePostDetail = () => {
    selectedPostSummary.value = null;
    selectedPostDetail.value = null;
    postDetailError.value = '';
    postDetailLoading.value = false;
};

const editPost = async (uuid, originView = '') => {
    draftSaving.value = true;
    draftError.value = '';
    contextualEditOrigin.value = originView && originView !== 'posts' ? originView : '';
    activeView.value = 'posts';
    activePostsMode.value = 'compose';

    try {
        const detail = await invoke('post_detail', { uuid });
        const original = detail.versions.find((version) => version.is_original) || detail.versions[0];
        const firstContent = original?.content?.[0] || { body: '', media: [] };

        editingPostUuid.value = detail.uuid;
        draftBody.value = firstContent.body || '';
        draftMediaIds.value = firstContent.media || [];
        draftExternalMedia.value = firstContent.external_media || [];
        draftAccountIds.value = detail.accounts;
        activeDraftVersion.value = 0;
        draftAccountBodies.value = Object.fromEntries(
            detail.versions
                .filter((version) => !version.is_original && version.account_id)
                .map((version) => {
                    const content = version.content?.[0] || { body: '' };

                    return [version.account_id, content.body || ''];
                }),
        );
        draftTagIds.value = detail.tags;
        draftScheduledAt.value = toDateTimeLocal(detail.scheduled_at);
    } catch (error) {
        draftError.value = String(error);
    } finally {
        draftSaving.value = false;
    }
};

const editSelectedPostFromDetail = async () => {
    const uuid = selectedPostSummary.value?.uuid;
    const originView = activeView.value;

    if (!uuid) {
        return;
    }

    closePostDetail();
    await editPost(uuid, originView);
};

const returnToContextualEditOrigin = () => {
    if (!contextualEditOrigin.value) {
        return;
    }

    activeView.value = contextualEditOrigin.value;
    contextualEditOrigin.value = '';
};

const duplicatePost = async (uuid) => {
    draftSaving.value = true;
    draftError.value = '';

    try {
        await invoke('duplicate_post', { uuid });
        resetDraftEditor();
        await load();
    } catch (error) {
        draftError.value = String(error);
    } finally {
        draftSaving.value = false;
    }
};

const validatePost = async (uuid) => {
    validationRunning.value = true;
    validationError.value = '';

    try {
        validationReport.value = await invoke('validate_post', { uuid });
    } catch (error) {
        validationReport.value = null;
        validationError.value = String(error);
    } finally {
        validationRunning.value = false;
    }
};

const deletePost = async (uuid) => {
    if (!await requestConfirmation({
        title: 'Delete this post?',
        description: 'The post will be removed from Dust Wave Social and any queued publishing work for it will be cancelled.',
        confirmLabel: 'Delete post',
        danger: true,
    })) {
        return;
    }

    draftSaving.value = true;
    draftError.value = '';

    try {
        await invoke('delete_post', { uuid });
        if (editingPostUuid.value === uuid) {
            resetDraftEditor();
        }
        if (selectedPostSummary.value?.uuid === uuid) {
            closePostDetail();
        }
        await load();
    } catch (error) {
        draftError.value = String(error);
    } finally {
        draftSaving.value = false;
    }
};

const bulkDeletePosts = async () => {
    if (!selectedPostUuids.value.length) {
        draftError.value = 'Select at least one post';
        return;
    }

    if (!await requestConfirmation({
        title: `Delete ${selectedPostUuids.value.length} selected post(s)?`,
        description: 'The selected posts will be removed from Dust Wave Social and their queued publishing work will be cancelled.',
        confirmLabel: 'Delete selected posts',
        danger: true,
    })) {
        return;
    }

    draftSaving.value = true;
    draftError.value = '';

    try {
        bulkDeleteSummary.value = await invoke('bulk_delete_posts', {
            request: {
                uuids: selectedPostUuids.value,
            },
        });
        if (selectedPostUuids.value.includes(editingPostUuid.value)) {
            resetDraftEditor();
        }
        if (selectedPostSummary.value && selectedPostUuids.value.includes(selectedPostSummary.value.uuid)) {
            closePostDetail();
        }
        selectedPostUuids.value = [];
        await load();
    } catch (error) {
        bulkDeleteSummary.value = null;
        draftError.value = String(error);
    } finally {
        draftSaving.value = false;
    }
};

const openPostScheduleEditor = (post) => {
    if (editingSchedulePostUuid.value && editingSchedulePostUuid.value !== post.uuid) {
        delete postScheduleDrafts.value[editingSchedulePostUuid.value];
    }

    postScheduleDrafts.value[post.uuid] = post.status === 'failed'
        ? ''
        : postScheduleDrafts.value[post.uuid] || toDateTimeLocal(post.scheduled_at);
    editingSchedulePostUuid.value = post.uuid;
};

const closePostScheduleEditor = () => {
    if (editingSchedulePostUuid.value) {
        delete postScheduleDrafts.value[editingSchedulePostUuid.value];
    }

    editingSchedulePostUuid.value = '';
};

const schedulePost = async (post) => {
    const scheduledAt = postScheduleDrafts.value[post.uuid] || '';

    if (!scheduledAt) {
        scheduleError.value = 'Schedule time is required';
        return;
    }

    scheduleSaving.value = true;
    scheduleError.value = '';

    try {
        await invoke(post.status === 'failed' ? 'retry_failed_post' : 'schedule_post', {
            uuid: post.uuid,
            schedule: {
                scheduled_at: new Date(scheduledAt).toISOString(),
            },
        });

        delete postScheduleDrafts.value[post.uuid];
        editingSchedulePostUuid.value = '';
        await load();
    } catch (error) {
        scheduleError.value = String(error);
    } finally {
        scheduleSaving.value = false;
    }
};

const retryFailedPostNow = async (post) => {
    if (!await requestConfirmation({
        title: 'Retry this post now?',
        description: `“${post.preview || 'Untitled post'}” may publish immediately to every account assigned to it.`,
        confirmLabel: 'Retry now',
    })) {
        return;
    }

    scheduleSaving.value = true;
    scheduleError.value = '';

    try {
        await invoke('retry_failed_post', {
            uuid: post.uuid,
            schedule: {
                scheduled_at: new Date().toISOString(),
            },
        });

        delete postScheduleDrafts.value[post.uuid];
        if (editingSchedulePostUuid.value === post.uuid) {
            editingSchedulePostUuid.value = '';
        }
        await load();
    } catch (error) {
        scheduleError.value = String(error);
    } finally {
        scheduleSaving.value = false;
    }
};

const runDueJobs = async () => {
    if (workerRunning.value) {
        return;
    }

    workerRunning.value = true;

    try {
        const summary = await invoke('run_due_jobs', {
            now: new Date().toISOString(),
            limit: 10,
        });
        if (summary?.reserved > 0) {
            await notifyWorkerRun(summary);
            await load();
        }
    } catch (error) {
        loadError.value = String(error);
    } finally {
        workerRunning.value = false;
    }
};

const runAutoWorkerTick = () => {
    if (workerRunning.value) {
        return;
    }

    runDueJobs();
};

const runAutoMaintenanceTick = () => {
    if (desktopMaintenanceRunning.value) {
        return;
    }

    runDesktopMaintenance({ background: true });
};

const startWorkerLoop = () => {
    if (!workerIntervalId) {
        workerIntervalId = window.setInterval(runAutoWorkerTick, WORKER_POLL_MS);
    }

    if (!maintenanceIntervalId) {
        maintenanceIntervalId = window.setInterval(runAutoMaintenanceTick, MAINTENANCE_POLL_MS);
    }
};

const stopWorkerLoop = () => {
    if (workerIntervalId) {
        window.clearInterval(workerIntervalId);
        workerIntervalId = null;
    }

    if (maintenanceIntervalId) {
        window.clearInterval(maintenanceIntervalId);
        maintenanceIntervalId = null;
    }
};

const postTime = (post) => {
    const value = post.scheduled_at || post.published_at || post.updated_at;

    if (!value) {
        return '';
    }

    return value.slice(11, 16);
};

watch(activeView, async (view) => {
    if (view === 'posts' || view === 'calendar') {
        await loadPostQuery();
    }

    if (view === 'reports') {
        await loadReport();
    }
});

watch(
    [
        editingPostUuid,
        draftBody,
        draftAccountIds,
        draftAccountBodies,
        activeDraftVersion,
        draftMediaIds,
        draftExternalMedia,
        draftTagIds,
        draftScheduledAt,
    ],
    persistComposerDraft,
    { deep: true },
);

watch(
    draftAccountIds,
    (accountIds) => {
        const selected = new Set(accountIds.map((id) => Number(id)));
        const nextBodies = { ...draftAccountBodies.value };
        let changed = false;

        Object.keys(nextBodies).forEach((accountId) => {
            if (!selected.has(Number(accountId))) {
                delete nextBodies[accountId];
                changed = true;
            }
        });

        if (changed) {
            draftAccountBodies.value = nextBodies;
        }

        if (Number(activeDraftVersion.value) !== 0 && !selected.has(Number(activeDraftVersion.value))) {
            activeDraftVersion.value = 0;
        }
    },
    { deep: true },
);

watch(
    [activeDraftVersion, activeDraftBody],
    () => {
        const editor = draftEditor.value;

        if (!editor) {
            return;
        }

        const content = normalizeEditorContent(activeDraftBody.value);

        if (editor.getHTML() !== content) {
            editor.commands.setContent(content, false);
        }

        emojiPickerOpen.value = false;
    },
    { flush: 'post' },
);

const postWindowLabel = computed(() => {
    const window = postQuery.value.calendar_window;

    if (!window) {
        return '';
    }

    return `${window.start_date} to ${window.end_date}`;
});

onMounted(async () => {
    restoreComposerDraft();
    await load();
    startWorkerLoop();
    runAutoWorkerTick();
    runAutoMaintenanceTick();
});

onUnmounted(() => {
    confirmationResolver?.(false);
    confirmationResolver = null;
    draftEditor.value?.destroy();
    stopWorkerLoop();
});
</script>

<template>
    <main class="shell">
        <aside class="sidebar">
            <div class="sidebar-brand">
                <div class="brand-mark">
                    <img :src="dustWaveSquareLogoUrl" alt="" />
                </div>
                <div>
                    <p class="brand-name">Dust Wave</p>
                    <p class="brand-subtitle">Social desk</p>
                </div>
            </div>
            <nav class="sidebar-nav" aria-label="Main navigation">
                <section v-for="group in navigationGroups" :key="group.label" class="sidebar-nav-group">
                    <p>{{ group.label }}</p>
                    <button
                        v-for="view in group.views"
                        :key="view.id"
                        type="button"
                        :class="{ 'is-active': activeView === view.id }"
                        :aria-current="activeView === view.id ? 'page' : undefined"
                        @click="activeView = view.id"
                    >
                        <span>{{ view.label }}</span>
                        <small>{{ navigationBadge(view.id) }}</small>
                    </button>
                </section>
            </nav>
        </aside>

        <section class="workspace">
            <header class="topbar">
                <div class="topbar-copy">
                    <p class="section-label">{{ activeViewDefinition.section }}</p>
                    <h1>{{ activeViewDefinition.label }}</h1>
                    <p class="topbar-description">{{ activeViewDefinition.description }}</p>
                </div>
                <div class="topbar-actions">
                    <UpdateStatusButton
                        :checking="softwareUpdateChecking"
                        :installing="softwareUpdateInstalling"
                        :status="softwareUpdateStatus"
                        :progress="softwareUpdateProgress"
                        :error="softwareUpdateError"
                        :available="softwareUpdateAvailable"
                        @activate="checkOrInstallSoftwareUpdate"
                    />
                    <div class="status-pill">Local workspace</div>
                </div>
            </header>

            <div v-if="loadError" class="error-panel">
                {{ loadError }}
            </div>

            <template v-else>
                <section v-if="attentionNotices.length" class="attention-strip" aria-label="Workspace attention">
                    <article
                        v-for="notice in attentionNotices"
                        :key="notice.title"
                        :class="['attention-card', `is-${notice.severity}`]"
                    >
                        <div>
                            <strong>{{ notice.title }}</strong>
                            <small>{{ notice.detail }}</small>
                        </div>
                        <button type="button" class="inline-button" @click="openAttentionNotice(notice)">
                            Review
                        </button>
                    </article>
                </section>

                <section v-if="activeView === 'dashboard'" class="summary-grid">
                    <article class="summary-card">
                        <span class="metric">{{ dashboard?.accounts?.authorized ?? 0 }}/{{ dashboard?.accounts?.total ?? 0 }}</span>
                        <span class="metric-label">connected accounts</span>
                    </article>
                    <article class="summary-card">
                        <span class="metric">{{ dashboard?.posts?.scheduled ?? 0 }}</span>
                        <span class="metric-label">scheduled posts</span>
                    </article>
                    <article class="summary-card">
                        <span class="metric">{{ dashboard?.posts?.published ?? 0 }}</span>
                        <span class="metric-label">published posts</span>
                    </article>
                    <article class="summary-card">
                        <span class="metric">{{ dashboard?.posts?.failed ?? 0 }}</span>
                        <span class="metric-label">failed posts</span>
                    </article>
                </section>

                <section v-if="activeView === 'dashboard' && snapshot.accounts.length" class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>Account Analytics</h2>
                            <p>Review provider metrics by connected account and reporting period.</p>
                        </div>
                        <div class="dashboard-report-controls">
                            <div class="dashboard-account-picker" aria-label="Report account">
                                <button
                                    v-for="account in snapshot.accounts"
                                    :key="account.uuid"
                                    type="button"
                                    :class="['dashboard-account-button', { 'is-active': Number(reportAccountId) === Number(account.id) }]"
                                    :disabled="reportLoading"
                                    :title="account.name || account.username || providerDisplayName(account.provider)"
                                    @click="loadReport(account.id)"
                                >
                                    <span class="dashboard-account-avatar">
                                        <img
                                            v-if="account.avatar_path"
                                            :src="mediaAssetUrl(account.avatar_path)"
                                            :alt="account.name || account.username || providerDisplayName(account.provider)"
                                        />
                                        <span v-else>{{ accountInitials(account) }}</span>
                                    </span>
                                    <span>{{ providerDisplayName(account.provider) }}</span>
                                </button>
                            </div>
                            <div class="dashboard-period-tabs" role="tablist" aria-label="Report period">
                                <button
                                    v-for="period in ['7_days', '30_days', '90_days']"
                                    :key="period"
                                    type="button"
                                    :class="{ 'is-active': reportPeriod === period }"
                                    :disabled="reportLoading"
                                    role="tab"
                                    :aria-selected="reportPeriod === period"
                                    @click="reportPeriod = period; loadReport()"
                                >
                                    {{ period.replace('_', ' ') }}
                                </button>
                            </div>
                        </div>
                    </div>
                    <div v-if="reportLoading" class="form-note">Loading report</div>
                    <div v-else-if="reportError" class="form-error">{{ reportError }}</div>
                    <div v-else-if="!report" class="empty-row">No account report available yet</div>
                    <div v-else class="report-layout">
                        <div class="report-provider-panel">
                            <div class="report-kicker">
                                {{ activeReportAccount?.name || activeReportAccount?.username || report.provider }} · {{ report.period }}
                            </div>
                            <div v-if="providerReportCards.length" class="report-card-grid">
                                <article v-for="metric in providerReportCards" :key="metric.key" class="report-card">
                                    <span>{{ metric.label }}</span>
                                    <strong>{{ metric.value }}</strong>
                                    <small>{{ metric.description }}</small>
                                </article>
                            </div>
                            <div v-else class="empty-row">No metrics for this provider</div>
                        </div>
                        <div class="audience-panel">
                            <div class="report-kicker">Audience</div>
                            <div v-if="reportAudienceSummary" class="report-chart-summary">
                                <div>
                                    <span>Selected</span>
                                    <strong>{{ formatNumber(activeReportAudiencePoint?.value) }}</strong>
                                    <small>{{ activeReportAudiencePoint?.label }}</small>
                                </div>
                                <div>
                                    <span>Change</span>
                                    <strong>{{ formatDelta(reportAudienceSummary.change) }}</strong>
                                    <small>{{ report.period }}</small>
                                </div>
                                <div>
                                    <span>High</span>
                                    <strong>{{ formatNumber(reportAudienceSummary.high) }}</strong>
                                    <small>Peak day</small>
                                </div>
                                <div>
                                    <span>Average</span>
                                    <strong>{{ formatNumber(reportAudienceSummary.average) }}</strong>
                                    <small>Daily mean</small>
                                </div>
                            </div>
                            <AudienceLineChart
                                v-if="reportAudiencePoints.length"
                                :points="reportAudiencePoints"
                                :active-index="activeReportAudienceIndex"
                                @select="activeReportAudienceIndex = $event"
                            />
                            <div v-else class="empty-row">No audience data for this period</div>
                        </div>
                    </div>
                </section>

                <section v-if="activeView === 'dashboard' && dashboard" class="dashboard-grid">
                    <article class="snapshot-list">
                        <header>
                            <div>
                                <h3>Upcoming</h3>
                                <small>{{ dashboard.posts.publishing }} publishing · {{ dashboard.jobs.pending }} queued</small>
                            </div>
                            <span>{{ dashboard.upcoming_posts.length }}</span>
                        </header>
                        <div v-if="!dashboard.upcoming_posts.length" class="empty-row">No scheduled posts</div>
                        <div v-for="post in dashboard.upcoming_posts" :key="post.uuid" class="snapshot-row">
                            <div>
                                <strong>{{ post.preview || 'Untitled post' }}</strong>
                                <small>{{ post.scheduled_at || post.updated_at }} · {{ post.account_count }} account(s)</small>
                            </div>
                            <span class="mini-state">{{ post.status }}</span>
                        </div>
                    </article>

                    <article class="snapshot-list">
                        <header>
                            <div>
                                <h3>Needs Attention</h3>
                                <small>{{ dashboard.accounts.unauthorized }} unauthorized · {{ dashboard.jobs.failed }} failed items</small>
                            </div>
                            <span>{{ dashboard.failed_posts.length }}</span>
                        </header>
                        <div v-if="!dashboard.failed_posts.length" class="empty-row">No failed posts</div>
                        <div v-for="post in dashboard.failed_posts" :key="post.uuid" class="snapshot-row">
                            <div>
                                <strong>{{ post.preview || 'Untitled post' }}</strong>
                                <small>{{ post.updated_at }} · {{ post.account_count }} account(s)</small>
                            </div>
                            <span class="mini-state is-error">{{ post.status }}</span>
                        </div>
                    </article>

                    <article class="snapshot-list">
                        <header>
                            <div>
                                <h3>Providers</h3>
                                <small>{{ dashboard.accounts.providers }} active provider(s)</small>
                            </div>
                            <span>{{ dashboard.providers.length }}</span>
                        </header>
                        <div v-if="!dashboard.providers.length" class="empty-row">No accounts connected</div>
                        <div v-for="provider in dashboard.providers" :key="provider.provider" class="snapshot-row">
                            <div>
                                <strong>{{ provider.provider }}</strong>
                                <small>{{ provider.authorized_accounts }}/{{ provider.accounts }} authorized</small>
                            </div>
                            <span class="mini-state">{{ provider.accounts }}</span>
                        </div>
                    </article>
                </section>

                <section v-if="activeView === 'system' && health" class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>System Health</h2>
                            <p>Operational status from local accounts, queued publishing, provider limits, and active service credentials.</p>
                        </div>
                        <div class="health-heading-actions">
                            <button
                                type="button"
                                :disabled="desktopMaintenanceRunning || maintenanceRunning"
                                @click="runDesktopMaintenance()"
                            >
                                Run Maintenance
                            </button>
                            <details class="system-action-menu">
                                <summary>More actions</summary>
                                <div class="system-action-menu-content">
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="maintenanceRunning || desktopMaintenanceRunning"
                                        @click="clearResolvedSystemState"
                                    >
                                        Clear Resolved State
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="staleRecoveryRunning"
                                        @click="recoverStaleProcessingJobs"
                                    >
                                        Recover Stale Jobs
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="failedImportRetryRunning || health.counts.failed_jobs === 0"
                                        @click="retryFailedAccountImports"
                                    >
                                        Retry Failed Imports
                                    </button>
                                    <button type="button" class="inline-button" @click="copySystemStatus">
                                        Copy Info
                                    </button>
                                    <button type="button" class="inline-button" @click="copyAppDataPath">
                                        Copy App Data Path
                                    </button>
                                </div>
                            </details>
                            <span :class="['status-pill', health.status !== 'ok' ? 'is-warning' : '']">
                                {{ health.status.replaceAll('_', ' ') }}
                            </span>
                        </div>
                    </div>
                    <div class="health-grid">
                        <div>
                            <span class="database-label">Unauthorized</span>
                            <strong>{{ health.counts.unauthorized_accounts }}</strong>
                        </div>
                        <div>
                            <span class="database-label">Failed posts</span>
                            <strong>{{ health.counts.failed_posts }}</strong>
                        </div>
                        <div>
                            <span class="database-label">Queued work</span>
                            <strong>{{ health.counts.pending_jobs }}</strong>
                        </div>
                        <div>
                            <span class="database-label">Processing</span>
                            <strong>{{ health.counts.processing_jobs }}</strong>
                        </div>
                        <div>
                            <span class="database-label">Failed work</span>
                            <strong>{{ health.counts.failed_jobs }}</strong>
                        </div>
                        <div>
                            <span class="database-label">Provider limits</span>
                            <strong>{{ health.counts.rate_limits }}</strong>
                        </div>
                    </div>
                    <div v-if="maintenanceSummary" class="form-note">
                        Cleared {{ maintenanceSummary.completed_jobs_deleted }} completed work item(s),
                        {{ maintenanceSummary.cancelled_jobs_deleted }} cancelled work item(s), and
                        {{ maintenanceSummary.expired_rate_limits_cleared }} expired provider limit(s)
                    </div>
                    <div v-if="desktopMaintenanceSummary" class="form-note">
                        Maintenance cleared {{ desktopMaintenanceSummary.resolved_state.completed_jobs_deleted }} completed work item(s),
                        {{ desktopMaintenanceSummary.resolved_state.cancelled_jobs_deleted }} cancelled work item(s),
                        {{ desktopMaintenanceSummary.resolved_state.expired_rate_limits_cleared }} expired provider limit(s), and
                        {{ desktopMaintenanceSummary.media.deleted }} orphaned media file(s)
                        <template v-if="autoMaintenanceLastRun"> · last check {{ autoMaintenanceLastRun }}</template>
                    </div>
                    <div v-if="staleRecoverySummary" class="form-note">
                        Requeued {{ staleRecoverySummary.requeued_jobs }} stale processing item(s)
                    </div>
                    <div v-if="failedImportRetryJobs.length" class="form-note">
                        Requeued {{ failedImportRetryJobs.length }} failed account import job(s)
                    </div>
                    <div v-if="systemStatusCopied" class="form-note">System status copied</div>
                    <div v-if="appDataPathCopied" class="form-note">App data path copied</div>
                    <div v-if="maintenanceError" class="form-error">{{ maintenanceError }}</div>
                    <div v-if="desktopMaintenanceError" class="form-error">{{ desktopMaintenanceError }}</div>
                    <div v-if="desktopNotificationError" class="form-error">{{ desktopNotificationError }}</div>
                    <div v-if="staleRecoveryError" class="form-error">{{ staleRecoveryError }}</div>
                    <div v-if="failedImportRetryError" class="form-error">{{ failedImportRetryError }}</div>
                    <div v-if="systemStatusCopyError" class="form-error">{{ systemStatusCopyError }}</div>
                    <div v-if="!health.issues.length" class="empty-row">No health issues detected</div>
                    <div v-else class="health-issues">
                        <div v-for="issue in health.issues" :key="`${issue.title}-${issue.detail}`" class="health-issue">
                            <span :class="['mini-state', issue.severity === 'error' ? 'is-error' : 'is-muted']">
                                {{ issue.severity }}
                            </span>
                            <div>
                                <strong>{{ issue.title }}</strong>
                                <small>{{ issue.detail }}</small>
                            </div>
                        </div>
                    </div>
                </section>

                <details v-if="activeView === 'system'" class="system-disclosure">
                    <summary>
                        <span>Technical details</span>
                        <small>Runtime, media tools, storage, and workspace diagnostics</small>
                    </summary>
                <section class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>Technical Details</h2>
                            <p>Desktop runtime details used for support and local troubleshooting.</p>
                        </div>
                    </div>
                    <div class="system-detail-grid">
                        <div v-for="row in systemTechnicalRows" :key="row.label" class="system-detail-row">
                            <span>{{ row.label }}</span>
                            <strong>{{ row.value }}</strong>
                        </div>
                    </div>
                </section>
                </details>

                <details v-if="activeView === 'system'" class="system-disclosure">
                    <summary>
                        <span>Backup and restore</span>
                        <small>Protect or replace the local workspace</small>
                    </summary>
                    <BackupRestorePanel
                        v-model:restore-path="restoreBackupPath"
                        :backup-running="backupRunning"
                        :backup-error="backupError"
                        :backup-summary="backupSummary"
                        :restore-running="restoreRunning"
                        :restore-error="restoreError"
                        :restore-summary="restoreSummary"
                        @backup="createLocalBackup"
                        @choose-restore="chooseRestoreBackupPath"
                        @restore="restoreLocalBackup"
                    />
                </details>

                <details v-if="activeView === 'system'" class="system-disclosure">
                    <summary>
                        <span>Software updates</span>
                        <small>Check, verify, install, and restart</small>
                    </summary>
                    <SoftwareUpdatesPanel
                        :checking="softwareUpdateChecking"
                        :installing="softwareUpdateInstalling"
                        :status="softwareUpdateStatus"
                        :progress="softwareUpdateProgress"
                        :error="softwareUpdateError"
                        :available="softwareUpdateAvailable"
                        :badge="softwareUpdateBadge"
                        @check="checkSoftwareUpdate"
                        @install="installSoftwareUpdate"
                    />
                </details>

                <details v-if="activeView === 'system'" class="system-disclosure">
                    <summary>
                        <span>System logs</span>
                        <small>Review, export, or clear redacted support logs</small>
                    </summary>
                <section class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>System Logs</h2>
                            <p>Operational log entries are stored locally, redacted for support, and can be exported or cleared.</p>
                        </div>
                        <div class="health-heading-actions">
                            <button type="button" class="inline-button" :disabled="systemLogRunning" @click="refreshSystemLogs">
                                Refresh
                            </button>
                            <button type="button" class="inline-button" :disabled="systemLogRunning" @click="exportSystemLog">
                                Export Log
                            </button>
                            <button type="button" class="danger-inline-button" :disabled="systemLogRunning" @click="clearSystemLogs">
                                Clear Log
                            </button>
                        </div>
                    </div>
                    <div v-if="systemLogExport" class="form-note">
                        Exported {{ formatBytes(systemLogExport.bytes) }} to {{ systemLogExport.path }}
                    </div>
                    <div v-if="systemLogClearSummary" class="form-note">
                        Cleared {{ systemLogClearSummary.deleted_entries }} log entr{{ systemLogClearSummary.deleted_entries === 1 ? 'y' : 'ies' }}
                    </div>
                    <div v-if="systemLogError" class="form-error">{{ systemLogError }}</div>
                    <div v-if="!systemLogs.length" class="empty-row">No system logs recorded yet</div>
                    <div v-else class="system-log-list">
                        <article v-for="log in systemLogs" :key="log.name" class="system-log-card">
                            <div class="system-log-meta">
                                <strong>{{ log.name }}</strong>
                                <small>{{ log.entry_count }} entries · {{ formatBytes(log.bytes) }}</small>
                            </div>
                            <div v-if="log.error" class="form-warning">{{ log.error }}</div>
                            <textarea class="system-log-preview" :value="log.contents" readonly></textarea>
                        </article>
                    </div>
                </section>
                </details>

                <section v-if="activeView === 'reports'" class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>Analytics</h2>
                            <p>Audience and provider metrics for connected social accounts.</p>
                        </div>
                        <div class="report-selects">
                            <select
                                v-model="reportAccountId"
                                class="period-select"
                                aria-label="Report account"
                                :disabled="reportLoading || !snapshot.accounts.length"
                                @change="loadReport(reportAccountId)"
                            >
                                <option v-if="!snapshot.accounts.length" value="">Connect an account first</option>
                                <option v-for="account in snapshot.accounts" :key="account.uuid" :value="account.id">
                                    {{ account.provider }} · {{ account.username || account.name }}
                                </option>
                            </select>
                            <select
                                v-model="reportPeriod"
                                class="period-select"
                                aria-label="Report period"
                                :disabled="reportLoading || !snapshot.accounts.length"
                                @change="loadReport()"
                            >
                                <option value="7_days">7 days</option>
                                <option value="30_days">30 days</option>
                                <option value="90_days">90 days</option>
                            </select>
                        </div>
                    </div>
                    <div v-if="reportLoading" class="form-note">Loading report</div>
                    <div v-else-if="reportError" class="form-error">{{ reportError }}</div>
                    <div v-else-if="!report" class="empty-action-row">
                        <span>{{ snapshot.accounts.length ? 'No account report is available yet.' : 'Connect an account before viewing provider analytics.' }}</span>
                        <button
                            v-if="!snapshot.accounts.length"
                            type="button"
                            class="inline-button"
                            @click="activeView = 'connections'; activeConnectionTab = 'accounts'; openAddAccountModal()"
                        >
                            Add Account
                        </button>
                    </div>
                    <div v-else class="report-layout">
                        <div class="report-provider-panel">
                            <div class="report-kicker">{{ report.provider }} · {{ report.period }}</div>
                            <div v-if="report.provider === 'twitter' && report.tier === 'free'" class="form-warning">
                                Free-tier X reports may be limited.
                            </div>
                            <div v-if="providerReportCards.length" class="report-card-grid">
                                <article v-for="metric in providerReportCards" :key="metric.key" class="report-card">
                                    <span>{{ metric.label }}</span>
                                    <strong>{{ metric.value }}</strong>
                                    <small>{{ metric.description }}</small>
                                </article>
                            </div>
                            <div v-else-if="report.metrics.length" class="report-card-grid">
                                <article v-for="metric in report.metrics" :key="metric.key" class="report-card">
                                    <span>{{ metric.key.replaceAll('_', ' ') }}</span>
                                    <strong>{{ metric.value }}</strong>
                                </article>
                            </div>
                            <div v-else class="empty-row">No metrics for this provider</div>
                        </div>
                        <div class="audience-panel">
                            <div class="report-kicker">Audience</div>
                            <p>The number of followers per day during the selected period.</p>
                            <div v-if="reportAudienceSummary" class="report-chart-summary">
                                <div>
                                    <span>Selected</span>
                                    <strong>{{ formatNumber(activeReportAudiencePoint?.value) }}</strong>
                                    <small>{{ activeReportAudiencePoint?.label }}</small>
                                </div>
                                <div>
                                    <span>Change</span>
                                    <strong>{{ formatDelta(reportAudienceSummary.change) }}</strong>
                                    <small>{{ report.period }}</small>
                                </div>
                                <div>
                                    <span>High</span>
                                    <strong>{{ formatNumber(reportAudienceSummary.high) }}</strong>
                                    <small>Peak day</small>
                                </div>
                                <div>
                                    <span>Average</span>
                                    <strong>{{ formatNumber(reportAudienceSummary.average) }}</strong>
                                    <small>Daily mean</small>
                                </div>
                            </div>
                            <AudienceLineChart
                                v-if="reportAudiencePoints.length"
                                :points="reportAudiencePoints"
                                :active-index="activeReportAudienceIndex"
                                @select="activeReportAudienceIndex = $event"
                            />
                            <div v-else class="empty-row">No audience data for this period</div>
                        </div>
                    </div>
                </section>

                <section v-if="activeView === 'calendar'" class="panel">
                    <div class="panel-heading">
                        <div>
                            <h2>Calendar</h2>
                            <p>Review scheduled, published, failed, and draft posts by date, account, tag, and content.</p>
                        </div>
                        <div class="status-pill">{{ postQuery.total }} posts</div>
                    </div>
                    <div class="calendar-toolbar">
                        <div class="row-actions">
                            <button type="button" class="inline-button" :disabled="postQueryLoading" @click="moveCalendar(-1)">
                                Previous
                            </button>
                            <button type="button" class="inline-button" :disabled="postQueryLoading" @click="selectToday">
                                Today
                            </button>
                            <button type="button" class="inline-button" :disabled="postQueryLoading" @click="moveCalendar(1)">
                                Next
                            </button>
                        </div>
                        <strong>{{ calendarTitle }}</strong>
                        <div class="status-tabs is-compact" role="tablist" aria-label="Calendar range">
                            <button type="button" role="tab" :aria-selected="postFilter.calendar_type === 'month'" :class="{ 'is-active': postFilter.calendar_type === 'month' }" @click="postFilter.calendar_type = 'month'; applyPostFilters()">Month</button>
                            <button type="button" role="tab" :aria-selected="postFilter.calendar_type === 'week'" :class="{ 'is-active': postFilter.calendar_type === 'week' }" @click="postFilter.calendar_type = 'week'; applyPostFilters()">Week</button>
                            <button type="button" role="tab" :aria-selected="postFilter.calendar_type === 'day'" :class="{ 'is-active': postFilter.calendar_type === 'day' }" @click="postFilter.calendar_type = 'day'; applyPostFilters()">Day</button>
                        </div>
                    </div>
                    <form class="post-query-form is-calendar-filter" @submit.prevent="applyPostFilters">
                        <input v-model="postFilter.date" type="date" aria-label="Calendar date" />
                        <select v-model="postFilter.status" aria-label="Post status">
                            <option value="">Scheduled + history</option>
                            <option value="draft">Drafts</option>
                            <option value="scheduled">Scheduled</option>
                            <option value="published">Published</option>
                            <option value="failed">Failed</option>
                        </select>
                        <input v-model="postFilter.keyword" aria-label="Search post content" placeholder="Search content" />
                        <button type="submit" :disabled="postQueryLoading">Apply</button>
                    </form>
                    <div class="post-filter-popover">
                        <button type="button" class="post-filter-trigger" @click="postFilterOpen = !postFilterOpen">
                            Filters
                            <span v-if="postFilterTotal">{{ postFilterTotal }}</span>
                        </button>
                        <div v-if="postFilterOpen" class="post-filter-menu">
                            <header>
                                <strong>Filters</strong>
                                <button type="button" class="inline-button" :disabled="!postFilterTotal && !postFilter.keyword" @click="clearPostFilters">
                                    Clear filter
                                </button>
                            </header>
                            <section>
                                <strong>Labels</strong>
                                <div v-if="snapshot.tags.length" class="post-filter-options">
                                    <label
                                        v-for="tag in snapshot.tags"
                                        :key="tag.uuid"
                                        :class="{ 'is-selected': postFilterTagSelected(tag) }"
                                    >
                                        <input v-model="postFilter.tags" type="checkbox" :value="tag.id" />
                                        <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                        {{ tag.name }}
                                    </label>
                                </div>
                                <p v-else class="empty-inline">No labels found</p>
                            </section>
                            <section>
                                <strong>Accounts</strong>
                                <div v-if="snapshot.accounts.length" class="post-filter-options">
                                    <label
                                        v-for="account in snapshot.accounts"
                                        :key="account.uuid"
                                        :class="{ 'is-selected': postFilterAccountSelected(account) }"
                                    >
                                        <input v-model="postFilter.accounts" type="checkbox" :value="account.id" />
                                        <span>{{ providerDisplayName(account.provider) }}</span>
                                        {{ account.name || account.username || account.provider_id }}
                                    </label>
                                </div>
                                <p v-else class="empty-inline">No accounts found</p>
                            </section>
                        </div>
                    </div>
                    <div class="post-query-meta">
                        <span v-if="postWindowLabel">{{ postWindowLabel }}</span>
                        <span v-if="postQuery.has_failed_posts" class="mini-state is-muted">failed posts present</span>
                        <button type="button" class="inline-button" @click="createPostFromCalendarDate(postFilter.date)">
                            New Post
                        </button>
                        <button
                            v-if="selectedPostUuids.length"
                            type="button"
                            class="danger-inline-button"
                            :disabled="draftSaving"
                            @click="bulkDeletePosts"
                        >
                            Delete {{ selectedPostUuids.length }}
                        </button>
                    </div>
                    <div v-if="bulkDeleteSummary" class="form-note">
                        Deleted {{ bulkDeleteSummary.deleted }} post(s) · cancelled {{ bulkDeleteSummary.cancelled_jobs }} queued item(s)
                    </div>
                    <div v-if="postQueryError" class="form-error">{{ postQueryError }}</div>
                    <div v-if="postQueryLoading" class="form-note">Loading calendar</div>
                    <div v-if="postFilter.calendar_type === 'week'" class="calendar-week-schedule">
                        <div class="calendar-week-header">
                            <span></span>
                            <button
                                v-for="cell in calendarCells"
                                :key="cell.date"
                                type="button"
                                :class="{ 'is-selected': cell.is_selected }"
                                @click="selectCalendarDate(cell.date)"
                            >
                                <strong>{{ cell.weekday }}</strong>
                                <span>{{ cell.day_number }}</span>
                            </button>
                        </div>
                        <div class="calendar-week-body">
                            <template v-for="slot in calendarWeekSlots" :key="slot.value">
                                <div class="calendar-week-time">{{ slot.label }}</div>
                                <div
                                    v-for="cell in calendarCells"
                                    :key="`${cell.date}-${slot.value}`"
                                    class="calendar-week-slot"
                                >
                                    <button
                                        type="button"
                                        class="calendar-week-add"
                                        @click="createPostFromCalendarSlot(cell.date, slot.hour)"
                                    >
                                        {{ slot.label }}
                                    </button>
                                    <button
                                        v-for="post in calendarSlotPosts(cell.date, slot.hour)"
                                        :key="post.uuid"
                                        type="button"
                                        :class="['calendar-post-chip', `is-${post.status}`]"
                                        @click="openPostDetail(post)"
                                    >
                                        {{ postTime(post) }} · {{ post.preview || post.uuid }}
                                    </button>
                                </div>
                            </template>
                        </div>
                    </div>
                    <div v-if="postFilter.calendar_type === 'month'" class="calendar-weekday-row">
                        <span v-for="label in calendarWeekdayLabels" :key="label">{{ label }}</span>
                    </div>
                    <div v-if="calendarCells.length && postFilter.calendar_type !== 'week'" :class="['calendar-grid', `is-${postFilter.calendar_type}`]">
                        <article
                            v-for="cell in calendarCells"
                            :key="cell.date"
                            role="button"
                            tabindex="0"
                            :class="['calendar-cell', {
                                'is-selected': cell.is_selected,
                                'is-outside': !cell.is_current_month && postFilter.calendar_type === 'month',
                            }]"
                            @click="selectCalendarDate(cell.date)"
                            @keydown.enter.prevent="selectCalendarDate(cell.date)"
                            @keydown.space.prevent="selectCalendarDate(cell.date)"
                        >
                            <span class="calendar-cell-date">
                                <strong>{{ cell.day_number }}</strong>
                                <small>{{ cell.weekday }}</small>
                            </span>
                            <span v-if="!cell.posts.length" class="calendar-empty-slot">No posts</span>
                            <button
                                v-for="post in cell.posts.slice(0, 4)"
                                :key="post.uuid"
                                type="button"
                                :class="['calendar-post-chip', `is-${post.status}`]"
                                @click.stop="openPostDetail(post)"
                            >
                                {{ postTime(post) }} · {{ post.preview || post.uuid }}
                            </button>
                            <span v-if="cell.posts.length > 4" class="calendar-more-chip">
                                +{{ cell.posts.length - 4 }} more
                            </span>
                        </article>
                    </div>
                    <div v-if="!postQueryError && !postGroups.length" class="empty-row">No matching posts</div>
                    <div v-else-if="!postQueryError" class="agenda-list">
                        <section v-for="group in postGroups" :key="group.date" class="agenda-day">
                            <div class="agenda-date">{{ group.date }}</div>
                            <div class="agenda-posts">
                                <div v-for="post in group.posts" :key="post.uuid" class="agenda-post">
                                    <label class="post-select">
                                        <input v-model="selectedPostUuids" type="checkbox" :value="post.uuid" />
                                    </label>
                                    <div class="agenda-time">{{ postTime(post) }}</div>
                                    <div>
                                        <strong>{{ post.preview || post.uuid }}</strong>
                                        <small>{{ post.account_count }} accounts · {{ post.tag_count }} tags</small>
                                    </div>
                                    <span class="mini-state">{{ post.status }}</span>
                                    <button type="button" class="inline-button" :disabled="postDetailLoading" @click="openPostDetail(post)">
                                        View
                                    </button>
                                </div>
                            </div>
                        </section>
                    </div>
                </section>

                <section v-if="isWorkspaceView" class="panel">
                    <WorkspaceTabs
                        v-if="activeView === 'connections'"
                        v-model="activeConnectionTab"
                        :tabs="connectionWorkspaceTabs"
                        label="Connections area"
                    />
                    <div class="snapshot-layout">
                        <article v-if="activeView === 'connections' && activeConnectionTab === 'accounts'" class="snapshot-list">
                            <header>
                                <div>
                                    <h3>Connected accounts</h3>
                                    <small>{{ connectedImportAccountCount }} ready to import</small>
                                </div>
                                <div class="row-actions">
                                    <button
                                        type="button"
                                        class="inline-button"
                                        @click="copyAccountOnboardingTemplate"
                                    >
                                        {{ accountOnboardingCopied === 'template' ? 'Copied CSV' : 'Copy Intake CSV' }}
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        @click="copyAccountOnboardingPlan"
                                    >
                                        {{ accountOnboardingCopied === 'plan' ? 'Copied Plan' : 'Copy Plan' }}
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        @click="openAddAccountModal()"
                                    >
                                        Add Account
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="queueAllImportsRunning || connectedImportAccountCount === 0"
                                        @click="queueAllAccountImports"
                                    >
                                        Queue All Imports
                                    </button>
                                    <span>{{ snapshot.accounts.length }}</span>
                                </div>
                            </header>
                            <div
                                v-if="addAccountModalOpen"
                                class="modal-backdrop"
                                role="dialog"
                                aria-modal="true"
                                aria-labelledby="add-account-title"
                                @click.self="closeAddAccountModal"
                            >
                                <div class="account-add-modal">
                                    <header>
                                        <div>
                                            <h3 id="add-account-title">Add account</h3>
                                            <small>{{ activeAccountProvider ? 'Complete the selected provider setup.' : 'Choose the social provider you want to connect.' }}</small>
                                        </div>
                                        <div class="modal-header-actions">
                                            <button
                                                v-if="activeAccountProvider"
                                                type="button"
                                                class="inline-button"
                                                @click="activeAccountProvider = ''"
                                            >
                                                Back to providers
                                            </button>
                                            <button type="button" class="modal-close-button" aria-label="Close add account" @click="closeAddAccountModal">
                                                &times;
                                            </button>
                                        </div>
                                    </header>
                                    <div v-if="!activeAccountProvider" class="account-provider-choice-list">
                                        <button
                                            v-for="provider in accountProviderChoices"
                                            :key="provider.id"
                                            type="button"
                                            class="account-provider-choice"
                                            @click="activeAccountProvider = provider.id"
                                        >
                                            <span>
                                                <strong>{{ provider.label }}</strong>
                                                <small>{{ provider.description }}</small>
                                            </span>
                                            <span :class="['mini-state', provider.service && serviceReady(provider.service) ? 'is-ok' : 'is-muted']">
                                                {{ provider.service ? (serviceReady(provider.service) ? 'ready' : 'setup needed') : 'server app' }}
                                            </span>
                                        </button>
                                    </div>
                                    <div v-else class="account-provider-step">
                                    <div class="account-provider-grid">
                                <article v-if="activeAccountProvider === 'twitter'" class="provider-connect-card">
                                    <header>
                                        <strong>X / Twitter</strong>
                                        <span :class="['mini-state', serviceReady('twitter') ? 'is-ok' : 'is-muted']">
                                            {{ serviceReady('twitter') ? 'ready' : 'service missing' }}
                                        </span>
                                    </header>
                                    <div v-if="!serviceReady('twitter')" class="form-warning">
                                        X service credentials must be active before connecting accounts.
                                        <button type="button" class="inline-button" @click="showServiceTab('twitter')">Open Provider Setup</button>
                                    </div>
                                    <form class="twitter-oauth-form" @submit.prevent="connectTwitterAccount">
                                        <input v-model="twitterOAuthDraft.redirect_uri" placeholder="X redirect URI" />
                                        <input v-model="twitterOAuthDraft.code" placeholder="Authorization code" />
                                        <button type="button" :disabled="twitterOAuthSaving || !serviceReady('twitter')" @click="startTwitterOAuth">Start X</button>
                                        <button type="submit" :disabled="twitterOAuthSaving || !serviceReady('twitter')">Connect</button>
                                    </form>
                                    <div v-if="twitterOAuthStart" class="form-note">
                                        X OAuth started ·
                                        <button type="button" class="inline-button" @click="openOAuthUrl(twitterOAuthStart.auth_url, twitterOAuthError)">
                                            Authorize
                                        </button>
                                    </div>
                                    <div v-if="twitterOAuthError" class="form-error">{{ twitterOAuthError }}</div>
                                    <div v-if="twitterOAuthConnection" class="form-note">
                                        {{ twitterOAuthConnection.account.name }} connected on X
                                    </div>
                                </article>

                                <article v-if="activeAccountProvider === 'facebook'" class="provider-connect-card">
                                    <header>
                                        <strong>Facebook Page / Instagram</strong>
                                        <span :class="['mini-state', serviceReady('facebook') ? 'is-ok' : 'is-muted']">
                                            {{ serviceReady('facebook') ? 'ready' : 'service missing' }}
                                        </span>
                                    </header>
                                    <div v-if="!serviceReady('facebook')" class="form-warning">
                                        Facebook service credentials must be active before connecting Pages or Instagram accounts.
                                        <button type="button" class="inline-button" @click="showServiceTab('facebook')">Open Provider Setup</button>
                                    </div>
                                    <form class="facebook-oauth-form" @submit.prevent="exchangeFacebookOAuth">
                                        <input v-model="facebookOAuthDraft.redirect_uri" placeholder="Facebook redirect URI" />
                                        <input v-model="facebookOAuthDraft.code" placeholder="Authorization code" />
                                        <button type="button" :disabled="facebookOAuthSaving || !serviceReady('facebook')" @click="startFacebookOAuth">Start Facebook</button>
                                        <button type="submit" :disabled="facebookOAuthSaving || !serviceReady('facebook')">List Accounts</button>
                                    </form>
                                    <div v-if="facebookOAuthStart" class="form-note">
                                        Facebook OAuth started ·
                                        <button type="button" class="inline-button" @click="openOAuthUrl(facebookOAuthStart.auth_url, facebookOAuthError)">
                                            Authorize
                                        </button>
                                    </div>
                                    <div v-if="facebookOAuthError" class="form-error">{{ facebookOAuthError }}</div>
                                    <div v-if="facebookUserConnection" class="facebook-entity-selector">
                                        <div class="facebook-entity-toolbar">
                                            <small>{{ facebookUserConnection.user_name }} · {{ facebookOAuthDraft.selected_pages.length }}/{{ facebookUserConnection.pages.length }} selected</small>
                                            <div class="row-actions">
                                                <button type="button" class="inline-button" :disabled="facebookOAuthSaving" @click="selectAllFacebookPages">
                                                    Select All
                                                </button>
                                                <button type="button" class="inline-button" :disabled="facebookOAuthSaving" @click="clearFacebookPages">
                                                    Clear
                                                </button>
                                            </div>
                                        </div>
                                        <label
                                            v-for="page in facebookUserConnection.pages"
                                            :key="page.id"
                                            :class="['facebook-entity-card', { 'is-selected': facebookPageIsSelected(page) }]"
                                        >
                                            <input v-model="facebookOAuthDraft.selected_pages" type="checkbox" :value="page.id" />
                                            <span class="provider-preview-avatar">
                                                <img v-if="page.avatar_path" :src="mediaAssetUrl(page.avatar_path)" :alt="page.name" />
                                                <span v-else>{{ accountInitials({ name: page.name, provider: 'facebook_page' }) }}</span>
                                            </span>
                                            <span>
                                                <strong>{{ page.name }}</strong>
                                                <small>{{ page.username ? `@${page.username}` : page.id }}</small>
                                            </span>
                                            <span :class="['mini-state', facebookPageIsSelected(page) ? 'is-ok' : 'is-muted']">
                                                {{ facebookPageIsSelected(page) ? 'selected' : 'page' }}
                                            </span>
                                        </label>
                                        <button type="button" :disabled="facebookOAuthSaving" @click="connectFacebookPages">
                                            Save Pages
                                        </button>
                                    </div>
                                    <div v-if="facebookPageConnection" class="form-note">
                                        {{ facebookPageConnection.accounts.length }} Facebook Page account(s) connected
                                    </div>
                                    <div v-if="facebookUserConnection?.instagram_accounts?.length" class="facebook-entity-selector">
                                        <div class="facebook-entity-toolbar">
                                            <small>Instagram · {{ facebookOAuthDraft.selected_instagram_accounts.length }}/{{ facebookUserConnection.instagram_accounts.length }} selected</small>
                                            <div class="row-actions">
                                                <button type="button" class="inline-button" :disabled="facebookOAuthSaving" @click="selectAllInstagramAccounts">
                                                    Select All
                                                </button>
                                                <button type="button" class="inline-button" :disabled="facebookOAuthSaving" @click="clearInstagramAccounts">
                                                    Clear
                                                </button>
                                            </div>
                                        </div>
                                        <label
                                            v-for="account in facebookUserConnection.instagram_accounts"
                                            :key="account.id"
                                            :class="['facebook-entity-card', { 'is-selected': instagramAccountIsSelected(account) }]"
                                        >
                                            <input v-model="facebookOAuthDraft.selected_instagram_accounts" type="checkbox" :value="account.id" />
                                            <span class="provider-preview-avatar">
                                                <img v-if="account.avatar_path" :src="mediaAssetUrl(account.avatar_path)" :alt="account.name" />
                                                <span v-else>{{ accountInitials({ name: account.name, provider: 'instagram' }) }}</span>
                                            </span>
                                            <span>
                                                <strong>{{ account.name }}</strong>
                                                <small>{{ account.username ? `@${account.username}` : account.page_name }}</small>
                                            </span>
                                            <span :class="['mini-state', instagramAccountIsSelected(account) ? 'is-ok' : 'is-muted']">
                                                {{ instagramAccountIsSelected(account) ? 'selected' : 'instagram' }}
                                            </span>
                                        </label>
                                        <button type="button" :disabled="facebookOAuthSaving" @click="connectInstagramAccounts">
                                            Save Instagram
                                        </button>
                                    </div>
                                    <div v-if="instagramAccountConnection" class="form-note">
                                        {{ instagramAccountConnection.accounts.length }} Instagram account(s) connected
                                    </div>
                                </article>

                                <article v-if="activeAccountProvider === 'mastodon'" class="provider-connect-card">
                                    <header>
                                        <strong>Mastodon</strong>
                                        <span class="mini-state">Server app</span>
                                    </header>
                                    <form class="mastodon-app-form" @submit.prevent="registerMastodonApp">
                                        <input v-model="mastodonAppDraft.server" placeholder="Mastodon server" />
                                        <input v-model="mastodonAppDraft.client_name" placeholder="Client name" />
                                        <input v-model="mastodonAppDraft.website" placeholder="Website" />
                                        <button type="submit" :disabled="mastodonAppSaving">Register</button>
                                    </form>
                                    <div v-if="mastodonAppError" class="form-error">{{ mastodonAppError }}</div>
                                    <div v-if="mastodonAppRegistration" class="form-note">
                                        {{ mastodonAppRegistration.service_name }} ·
                                        <button type="button" class="inline-button" @click="openOAuthUrl(mastodonAppRegistration.auth_url, mastodonAppError)">
                                            Authorize
                                        </button>
                                    </div>
                                    <form class="mastodon-oauth-form" @submit.prevent="connectMastodonAccount">
                                        <input v-model="mastodonOAuthDraft.server" placeholder="Mastodon server" />
                                        <input v-model="mastodonOAuthDraft.code" placeholder="Authorization code" />
                                        <button type="submit" :disabled="mastodonOAuthSaving">Connect</button>
                                    </form>
                                    <div v-if="mastodonOAuthError" class="form-error">{{ mastodonOAuthError }}</div>
                                    <div v-if="mastodonOAuthConnection" class="form-note">
                                        {{ mastodonOAuthConnection.account.name }} connected on {{ mastodonOAuthConnection.server }}
                                    </div>
                                </article>

                                <article v-if="activeAccountProvider === 'tiktok'" class="provider-connect-card">
                                    <header>
                                        <strong>TikTok</strong>
                                        <span :class="['mini-state', serviceReady('tiktok') ? 'is-ok' : 'is-muted']">
                                            {{ serviceReady('tiktok') ? 'broker ready' : 'service missing' }}
                                        </span>
                                    </header>
                                    <div v-if="!serviceReady('tiktok')" class="form-warning">
                                        TikTok requires the client key, active service status, and HTTPS broker URL before analytics imports.
                                        <button type="button" class="inline-button" @click="showServiceTab('tiktok')">Open Provider Setup</button>
                                    </div>
                                    <div v-else class="form-note">
                                        Authorize in the broker, then paste the returned account values and broker credential here.
                                        <button type="button" class="inline-button" @click="openTikTokBrokerAuth">Open Broker Auth</button>
                                    </div>
                                    <form class="tiktok-connection-form" @submit.prevent="connectTikTokAccount">
                                        <input v-model="tiktokConnectionDraft.provider_id" placeholder="TikTok user ID" />
                                        <input v-model="tiktokConnectionDraft.name" placeholder="Display name" />
                                        <input v-model="tiktokConnectionDraft.username" placeholder="Username" />
                                        <input v-model="tiktokConnectionDraft.scopes" placeholder="Granted scopes" />
                                        <input v-model="tiktokConnectionDraft.broker_connection" type="password" autocomplete="new-password" placeholder="Broker connection credential" />
                                        <button type="submit" :disabled="tiktokConnectionSaving || !serviceReady('tiktok')">Connect</button>
                                    </form>
                                    <div class="form-note">
                                        Assisted publishing is available for MVP. Direct TikTok API publishing stays disabled until the app is approved for upload or publish scopes.
                                    </div>
                                    <div v-if="tiktokConnectionError" class="form-error">{{ tiktokConnectionError }}</div>
                                    <div v-if="tiktokConnection" class="form-note">
                                        {{ tiktokConnection.account.name }} connected on TikTok
                                    </div>
                                </article>
                                    </div>
                                </div>
                                </div>
                            </div>
                            <div v-if="accountError" class="form-error">{{ accountError }}</div>
                            <div v-if="accountOnboardingCopied" class="form-note">
                                {{ accountOnboardingCopied === 'template' ? 'Account intake CSV copied.' : 'Account onboarding plan copied.' }}
                            </div>
                            <div class="connected-account-grid">
                                <button type="button" class="add-account-card" @click="openAddAccountModal()">
                                    <span>+</span>
                                    <strong>Add account</strong>
                                    <small>Connect X, Facebook Page, Instagram, Mastodon, or TikTok.</small>
                                </button>
                                <article v-for="account in snapshot.accounts" :key="account.uuid" class="connected-account-card">
                                    <header>
                                        <span class="connected-account-avatar">
                                            <img
                                                v-if="account.avatar_path"
                                                :src="mediaAssetUrl(account.avatar_path)"
                                                :alt="account.name || account.username || providerDisplayName(account.provider)"
                                            />
                                            <span v-else>{{ accountInitials(account) }}</span>
                                            <i :class="{ 'is-warning': !account.authorized }"></i>
                                        </span>
                                        <span :class="['mini-state', account.authorized ? 'is-ok' : 'is-muted']">
                                            {{ account.authorized ? 'authorized' : 'needs auth' }}
                                        </span>
                                    </header>
                                    <div class="connected-account-copy">
                                        <strong>{{ account.name }}</strong>
                                        <small>{{ providerDisplayName(account.provider) }} · {{ previewHandle(account) || account.provider_id }}</small>
                                    </div>
                                    <div class="connected-account-actions">
                                        <button
                                            v-if="accountCanRefresh(account)"
                                            type="button"
                                            class="inline-button"
                                            :disabled="accountRefreshingUuid === account.uuid"
                                            @click="refreshAccount(account)"
                                        >
                                            Refresh
                                        </button>
                                        <button
                                            v-if="accountProviderOperation(account)?.importCommand"
                                            type="button"
                                            class="inline-button"
                                            :disabled="accountImportingUuid === account.uuid"
                                            @click="importAccountData(account)"
                                        >
                                            Import
                                        </button>
                                        <button
                                            v-if="accountImportSupported(account)"
                                            type="button"
                                            class="inline-button"
                                            :disabled="accountQueuingUuid === account.uuid"
                                            @click="queueAccountImport(account.uuid)"
                                        >
                                            Queue
                                        </button>
                                        <button type="button" class="danger-inline-button" :disabled="accountSaving" @click="deleteAccount(account.uuid)">
                                            Disconnect
                                        </button>
                                    </div>
                                </article>
                            </div>
                            <div v-if="mastodonImportSummary" class="form-note">
                                {{ mastodonImportSummary.account.name }} imported {{ mastodonImportSummary.imported_posts }} posts ·
                                {{ mastodonImportSummary.metric_days }} metric days
                            </div>
                            <div v-if="twitterImportSummary" class="form-note">
                                {{ twitterImportSummary.account.name }} imported {{ twitterImportSummary.imported_posts }} posts ·
                                {{ twitterImportSummary.metric_days }} metric days
                            </div>
                            <div v-if="facebookImportSummary" class="form-note">
                                {{ facebookImportSummary.account.name }} imported {{ facebookImportSummary.insight_rows }} insight rows
                            </div>
                            <div v-if="instagramImportSummary" class="form-note">
                                {{ instagramImportSummary.account.name }} imported {{ instagramImportSummary.imported_posts }} media item(s) ·
                                {{ instagramImportSummary.metric_days }} metric days
                            </div>
                            <div v-if="tiktokImportSummary" class="form-note">
                                {{ tiktokImportSummary.account.name }} imported {{ tiktokImportSummary.imported_posts }} videos ·
                                {{ tiktokImportSummary.metric_days }} metric days
                            </div>
                            <div v-if="queuedImportJob" class="form-note">
                                Queued account import
                            </div>
                            <div v-if="queuedImportBatch" class="form-note">
                                Queued {{ queuedImportBatch.queued_jobs }} account import(s) for {{ queuedImportBatch.eligible_accounts }} connected account(s) ·
                                skipped {{ queuedImportBatch.skipped_unauthorized + queuedImportBatch.skipped_unsupported }}
                            </div>
                            <div v-if="queuedImportError" class="form-error">{{ queuedImportError }}</div>
                        </article>

                        <article v-if="activeView === 'connections' && activeConnectionTab === 'services'" class="snapshot-list">
                            <header>
                                <div>
                                    <h3>Provider setup</h3>
                                    <small>{{ serviceReadinessSummary }}</small>
                                </div>
                                <div class="row-actions">
                                    <button
                                        type="button"
                                        class="inline-button"
                                        @click="copyProviderSetupBundle(false)"
                                    >
                                        {{ serviceSetupCopied === 'bundle:all' ? 'Copied Setup' : 'Copy All Setup' }}
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        @click="copyProviderSetupBundle(true)"
                                    >
                                        {{ serviceSetupCopied === 'bundle:missing' ? 'Copied Missing' : 'Copy Missing' }}
                                    </button>
                                    <span>{{ configuredCredentialCount }}/{{ credentialStatuses.length }} configured</span>
                                </div>
                            </header>
                            <div v-if="serviceSetupCopied === 'bundle:all' || serviceSetupCopied === 'bundle:missing'" class="form-note">
                                Provider setup packet copied without secret values.
                            </div>
                            <div class="service-tab-list" role="tablist" aria-label="Third party services">
                                <button
                                    v-for="service in serviceDefinitions"
                                    :key="service.id"
                                    type="button"
                                    :class="{ 'is-active': activeServiceTab === service.id }"
                                    :disabled="serviceSaving"
                                    role="tab"
                                    :aria-selected="activeServiceTab === service.id"
                                    @click="selectServiceTab(service.id)"
                                >
                                    {{ service.label }}
                                    <span :class="['mini-state', serviceReady(service.id) ? 'is-ok' : 'is-muted']">
                                        {{ serviceTabStatusText(service.id) }}
                                    </span>
                                </button>
                            </div>
                            <section class="service-provider-panel">
                                <header>
                                    <div>
                                        <strong>{{ activeServiceDefinition.label }}</strong>
                                        <small>{{ activeServiceDefinition.description }}</small>
                                    </div>
                                    <span :class="['mini-state', activeServiceIsReady ? 'is-ok' : 'is-muted']">
                                        {{ serviceStatusText(activeServiceDefinition.id) }}
                                    </span>
                                </header>
                                <div class="service-link-row">
                                    <button type="button" class="inline-button" @click="openServiceUrl(activeServiceDefinition.setupUrl)">
                                        Create App
                                    </button>
                                    <button
                                        v-if="activeServiceDefinition.docsUrl"
                                        type="button"
                                        class="inline-button"
                                        @click="openServiceUrl(activeServiceDefinition.docsUrl)"
                                    >
                                        Read Docs
                                    </button>
                                    <button
                                        v-if="activeServiceDefinition.setupFields?.length"
                                        type="button"
                                        class="inline-button"
                                        @click="copyServiceSetup(activeServiceDefinition)"
                                    >
                                        {{ serviceSetupCopied === `${activeServiceDefinition.id}:setup` ? 'Copied Setup' : 'Copy Setup' }}
                                    </button>
                                </div>
                                <div v-if="activeServiceDefinition.setupFields?.length" class="service-setup-grid">
                                    <div
                                        v-for="field in activeServiceDefinition.setupFields"
                                        :key="field.key"
                                        class="service-setup-item"
                                    >
                                        <span>{{ field.label }}</span>
                                        <code>{{ serviceSetupFieldValue(activeServiceDefinition, field) }}</code>
                                        <button
                                            type="button"
                                            class="inline-button"
                                            @click="copyServiceSetupField(activeServiceDefinition, field)"
                                        >
                                            {{ serviceSetupCopied === `${activeServiceDefinition.id}:${field.key}` ? 'Copied' : 'Copy' }}
                                        </button>
                                    </div>
                                </div>
                                <form
                                    class="service-settings-form"
                                    :aria-busy="serviceSaving"
                                    @submit.prevent="saveServiceSettings(activeServiceDefinition.id)"
                                >
                                    <p class="service-security-note">
                                        Credentials entered here are stored in macOS Keychain. Leave an available field blank to keep its current value.
                                    </p>
                                    <div class="service-field-list">
                                        <div
                                            v-for="credential in activeServiceDefinition.credentials"
                                            :key="credential.field"
                                            class="service-field-form"
                                        >
                                            <label :for="`service-${activeServiceDefinition.id}-${credential.field}`">
                                                <span>{{ credential.label }}</span>
                                                <small>{{ serviceCredentialInputHint(credential.field) }}</small>
                                            </label>
                                            <input
                                                :id="`service-${activeServiceDefinition.id}-${credential.field}`"
                                                v-model="serviceCredentialDrafts[activeServiceDefinition.id][credential.field]"
                                                :type="credential.secret ? 'password' : 'text'"
                                                :autocomplete="credential.autocomplete"
                                                :placeholder="activeServiceConfiguredFields.get(credential.field)?.configured ? 'Leave blank to keep saved value' : 'Enter value'"
                                                :disabled="serviceSaving"
                                                @input="clearServiceFeedback"
                                            />
                                        </div>
                                    </div>
                                    <div v-if="activeServiceDefinition.configuration.length" class="service-options-grid">
                                        <label v-for="configField in activeServiceDefinition.configuration" :key="configField.field">
                                            <span>{{ configField.label }}</span>
                                            <select
                                                v-if="configField.options?.length"
                                                v-model="serviceConfigurationDrafts[activeServiceDefinition.id][configField.field]"
                                                :disabled="serviceSaving"
                                                @change="clearServiceFeedback"
                                            >
                                                <option
                                                    v-for="option in configField.options"
                                                    :key="typeof option === 'string' ? option : option.value"
                                                    :value="typeof option === 'string' ? option : option.value"
                                                >
                                                    {{ typeof option === 'string' ? option : option.label }}
                                                </option>
                                            </select>
                                            <input
                                                v-else
                                                v-model="serviceConfigurationDrafts[activeServiceDefinition.id][configField.field]"
                                                :placeholder="configField.placeholder || ''"
                                                :disabled="serviceSaving"
                                                @input="clearServiceFeedback"
                                            />
                                        </label>
                                    </div>
                                    <div class="service-provider-actions">
                                        <label>
                                            <input
                                                v-model="serviceActiveDrafts[activeServiceDefinition.id]"
                                                type="checkbox"
                                                :disabled="serviceSaving"
                                                @change="clearServiceFeedback"
                                            />
                                            Active
                                        </label>
                                        <button type="submit" :disabled="serviceSaving">
                                            {{ serviceSaving ? `Saving ${activeServiceDefinition.label}…` : `Save ${activeServiceDefinition.label} Settings` }}
                                        </button>
                                        <button
                                            v-if="activeServiceDefinition.id === 'tiktok'"
                                            type="button"
                                            class="inline-button"
                                            :disabled="serviceSaving || !tiktokBrokerAuthUrl()"
                                            @click="openTikTokBrokerAuthFromServices"
                                        >
                                            Open Broker Auth
                                        </button>
                                    </div>
                                    <p v-if="serviceSaveMessage" class="form-note service-save-feedback" role="status" aria-live="polite">
                                        {{ serviceSaveMessage }}
                                    </p>
                                    <p v-if="serviceError" class="form-error service-save-feedback" role="alert">
                                        {{ serviceError }}
                                    </p>
                                </form>
                            </section>
                            <details class="credential-diagnostics">
                                <summary>Credential diagnostics · {{ configuredCredentialCount }}/{{ credentialStatuses.length }} configured</summary>
                            <div class="credential-grid">
                                <div v-for="status in credentialStatuses" :key="status.service" class="credential-card">
                                    <div class="credential-card-header">
                                        <strong>{{ status.label }}</strong>
                                        <span :class="['mini-state', status.configured ? 'is-ok' : 'is-muted']">
                                            {{ status.configured ? 'configured' : 'missing' }}
                                        </span>
                                    </div>
                                    <small>{{ status.group }} · {{ status.active ? 'active' : 'inactive' }}</small>
                                    <div class="credential-fields">
                                        <div v-for="field in status.fields" :key="field.field" class="credential-field">
                                            <span>{{ field.label }}</span>
                                            <span :class="['mini-state', field.configured ? 'is-ok' : 'is-muted']">
                                                {{ field.configured ? 'set' : field.env_vars.join(' or ') }}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            </details>
                        </article>

                        <article v-if="activeView === 'posts'" class="snapshot-list">
                            <header>
                                <h3>Post workspace</h3>
                                <span>{{ postQuery.total }}</span>
                            </header>
                            <WorkspaceTabs
                                v-model="activePostsMode"
                                :tabs="postWorkspaceTabs"
                                label="Post workspace"
                            />
                            <div v-show="activePostsMode === 'library'" class="post-library-controls">
                            <div class="status-tabs" role="tablist" aria-label="Post status">
                                <button type="button" role="tab" :aria-selected="!postFilter.status" :class="{ 'is-active': !postFilter.status }" @click="setPostStatusFilter('')">All</button>
                                <button type="button" role="tab" :aria-selected="postFilter.status === 'draft'" :class="{ 'is-active': postFilter.status === 'draft' }" @click="setPostStatusFilter('draft')">Drafts</button>
                                <button type="button" role="tab" :aria-selected="postFilter.status === 'scheduled'" :class="{ 'is-active': postFilter.status === 'scheduled' }" @click="setPostStatusFilter('scheduled')">Scheduled</button>
                                <button type="button" role="tab" :aria-selected="postFilter.status === 'published'" :class="{ 'is-active': postFilter.status === 'published' }" @click="setPostStatusFilter('published')">Published</button>
                                <button
                                    v-if="postQuery.has_failed_posts || postFilter.status === 'failed'"
                                    type="button"
                                    role="tab"
                                    :aria-selected="postFilter.status === 'failed'"
                                    :class="{ 'is-active': postFilter.status === 'failed' }"
                                    @click="setPostStatusFilter('failed')"
                                >
                                    Failed
                                </button>
                            </div>
                            <form class="post-query-form is-post-index" @submit.prevent="applyPostFilters">
                                <input v-model="postFilter.keyword" aria-label="Search post content" placeholder="Search content" />
                                <button type="submit" :disabled="postQueryLoading">Filter</button>
                            </form>
                            <div class="post-filter-popover">
                                <button type="button" class="post-filter-trigger" @click="postFilterOpen = !postFilterOpen">
                                    Filters
                                    <span v-if="postFilterTotal">{{ postFilterTotal }}</span>
                                </button>
                                <div v-if="postFilterOpen" class="post-filter-menu">
                                    <header>
                                        <strong>Filters</strong>
                                        <button type="button" class="inline-button" :disabled="!postFilterTotal && !postFilter.keyword" @click="clearPostFilters">
                                            Clear filter
                                        </button>
                                    </header>
                                    <section>
                                        <strong>Labels</strong>
                                        <div v-if="snapshot.tags.length" class="post-filter-options">
                                            <label
                                                v-for="tag in snapshot.tags"
                                                :key="tag.uuid"
                                                :class="{ 'is-selected': postFilterTagSelected(tag) }"
                                            >
                                                <input v-model="postFilter.tags" type="checkbox" :value="tag.id" />
                                                <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                                {{ tag.name }}
                                            </label>
                                        </div>
                                        <p v-else class="empty-inline">No labels found</p>
                                    </section>
                                    <section>
                                        <strong>Accounts</strong>
                                        <div v-if="snapshot.accounts.length" class="post-filter-options">
                                            <label
                                                v-for="account in snapshot.accounts"
                                                :key="account.uuid"
                                                :class="{ 'is-selected': postFilterAccountSelected(account) }"
                                            >
                                                <input v-model="postFilter.accounts" type="checkbox" :value="account.id" />
                                                <span>{{ providerDisplayName(account.provider) }}</span>
                                                {{ account.name || account.username || account.provider_id }}
                                            </label>
                                        </div>
                                        <p v-else class="empty-inline">No accounts found</p>
                                    </section>
                                </div>
                            </div>
                            </div>
                            <div v-show="activePostsMode === 'compose'" class="post-composer-workspace">
                            <form class="draft-form" @submit.prevent="saveDraftPost">
                                <div v-if="editingPostUuid" class="editor-state">
                                    <div class="editor-state-copy">
                                        <span>Editing {{ editingPostUuid }}</span>
                                        <small v-if="contextualEditOrigin">Opened from {{ contextualEditOriginLabel }}</small>
                                    </div>
                                    <div class="row-actions">
                                        <button
                                            v-if="contextualEditOrigin"
                                            type="button"
                                            class="inline-button"
                                            @click="returnToContextualEditOrigin"
                                        >
                                            Back to {{ contextualEditOriginLabel }}
                                        </button>
                                        <button type="button" class="inline-button" @click="resetDraftEditor">
                                            Cancel editing
                                        </button>
                                    </div>
                                </div>
                                <div class="composer-panel">
                                    <header class="composer-section-header">
                                        <div>
                                            <strong>Accounts</strong>
                                            <small>{{ selectedDraftAccounts.length }} selected</small>
                                        </div>
                                        <span class="mini-state">{{ draftVersionCount }} version{{ draftVersionCount === 1 ? '' : 's' }}</span>
                                    </header>
                                    <div v-if="snapshot.accounts.length" class="account-picker-grid" aria-label="Post accounts">
                                        <label
                                            v-for="account in snapshot.accounts"
                                            :key="account.uuid"
                                            :class="[
                                                'account-picker-card',
                                                {
                                                    'is-selected': isDraftAccountSelected(account),
                                                    'is-active': draftVersionAccountIds.has(Number(account.id)),
                                                    'is-disabled': draftAccountDisabledReason(account),
                                                },
                                            ]"
                                        >
                                            <input
                                                type="checkbox"
                                                :disabled="Boolean(draftAccountDisabledReason(account))"
                                                :checked="isDraftAccountSelected(account)"
                                                @change="toggleDraftAccount(account)"
                                            />
                                            <span class="account-picker-avatar">
                                                <img
                                                    v-if="account.avatar_path"
                                                    :src="mediaAssetUrl(account.avatar_path)"
                                                    :alt="account.name || account.username || providerDisplayName(account.provider)"
                                                />
                                                <span v-else>{{ accountInitials(account) }}</span>
                                            </span>
                                            <span class="account-picker-copy">
                                                <strong>{{ account.name || account.username || providerDisplayName(account.provider) }}</strong>
                                                <small>
                                                    {{ providerDisplayName(account.provider) }} ·
                                                    {{ draftAccountDisabledReason(account) || (draftVersionAccountIds.has(Number(account.id)) ? 'custom version' : previewHandle(account)) }}
                                                </small>
                                            </span>
                                        </label>
                                    </div>
                                    <p v-else class="empty-inline">No connected accounts</p>
                                    <div v-if="draftProviderWarnings.length" class="form-warning">
                                        <span v-for="warning in draftProviderWarnings" :key="warning">{{ warning }}</span>
                                    </div>
                                </div>
                                <div class="composer-panel">
                                    <header class="composer-section-header">
                                        <div>
                                            <strong>Versions</strong>
                                            <small>{{ activeDraftVersionTab.sublabel }}</small>
                                        </div>
                                        <span :class="['mini-state', { 'is-error': activeDraftOverLimit }]">{{ activeDraftCharacterLabel }}</span>
                                    </header>
                                    <div
                                        v-if="availableDraftVersionAccounts.length > 1 || (availableDraftVersionAccounts.length === 1 && draftVersionCount > 1)"
                                        class="composer-version-add"
                                    >
                                        <button
                                            type="button"
                                            class="inline-button"
                                            :disabled="!availableDraftVersionAccounts.length"
                                            @click="versionPickerOpen = !versionPickerOpen"
                                        >
                                            Create version
                                        </button>
                                        <div v-if="versionPickerOpen" class="composer-version-menu">
                                            <button
                                                v-for="account in availableDraftVersionAccounts"
                                                :key="account.uuid"
                                                type="button"
                                                @click="createDraftAccountVersion(account.id)"
                                            >
                                                <span class="account-picker-avatar">
                                                    <img
                                                        v-if="account.avatar_path"
                                                        :src="mediaAssetUrl(account.avatar_path)"
                                                        :alt="account.name || account.username || providerDisplayName(account.provider)"
                                                    />
                                                    <span v-else>{{ accountInitials(account) }}</span>
                                                </span>
                                                <span>
                                                    <strong>{{ account.name || account.username || providerDisplayName(account.provider) }}</strong>
                                                    <small>{{ providerDisplayName(account.provider) }}</small>
                                                </span>
                                            </button>
                                        </div>
                                    </div>
                                    <button
                                        v-else-if="availableDraftVersionAccounts.length === 1 && draftVersionCount === 1"
                                        type="button"
                                        class="inline-button"
                                        @click="createDraftAccountVersion(availableDraftVersionAccounts[0].id)"
                                    >
                                        Create version for {{ availableDraftVersionAccounts[0].name || availableDraftVersionAccounts[0].username || providerDisplayName(availableDraftVersionAccounts[0].provider) }}
                                    </button>
                                    <div class="composer-version-tabs" role="tablist" aria-label="Post versions">
                                        <div
                                            v-for="tab in draftVersionTabs"
                                            :key="tab.key"
                                            :class="['composer-version-tab-wrap', { 'is-active': Number(activeDraftVersionTab.id) === Number(tab.id) }]"
                                        >
                                            <button
                                                type="button"
                                                :class="['composer-version-tab', { 'is-active': Number(activeDraftVersionTab.id) === Number(tab.id) }]"
                                                role="tab"
                                                :aria-selected="Number(activeDraftVersionTab.id) === Number(tab.id)"
                                                @click="setActiveDraftVersion(tab.id)"
                                            >
                                                <span class="version-tab-main">
                                                    <span class="version-tab-label">{{ tab.label }}</span>
                                                    <small>{{ tab.sublabel }}</small>
                                                </span>
                                            </button>
                                            <button
                                                v-if="Number(tab.id) !== 0"
                                                type="button"
                                                class="version-remove-button"
                                                title="Remove version"
                                                :aria-label="`Remove ${tab.label} version`"
                                                @click="removeDraftAccountVersion(tab.id)"
                                            >
                                                &times;
                                            </button>
                                        </div>
                                    </div>
                                    <div class="version-editor">
                                        <span class="version-editor-header">
                                            <span>
                                                <strong>{{ activeDraftVersionTab.label }}</strong>
                                                <small>{{ activeDraftVersionTab.sublabel }}</small>
                                            </span>
                                            <span :class="['composer-counter', { 'is-error': activeDraftOverLimit }]">{{ activeDraftCharacterLabel }}</span>
                                        </span>
                                        <div class="rich-editor-shell">
                                            <div class="rich-editor-toolbar">
                                                <button type="button" :disabled="!draftEditor" title="Undo" @click="runDraftEditorCommand('undo')">
                                                    Undo
                                                </button>
                                                <button type="button" :disabled="!draftEditor" title="Redo" @click="runDraftEditorCommand('redo')">
                                                    Redo
                                                </button>
                                                <div class="emoji-picker-wrap">
                                                    <button type="button" :disabled="!draftEditor" title="Emoji" @click="emojiPickerOpen = !emojiPickerOpen">
                                                        Emoji
                                                    </button>
                                                    <div v-if="emojiPickerOpen" class="emoji-popover" aria-label="Emoji picker">
                                                        <EmojiPickerPanel @select="insertDraftEmoji" />
                                                    </div>
                                                </div>
                                            </div>
                                            <EditorContent v-if="draftEditor" :editor="draftEditor" class="desktop-rich-editor" />
                                            <textarea
                                                v-else
                                                v-model="activeDraftBody"
                                                maxlength="5000"
                                                rows="4"
                                                :placeholder="Number(activeDraftVersion) === 0 ? 'Write post copy' : `Override copy for ${activeDraftVersionTab.label}`"
                                            ></textarea>
                                        </div>
                                    </div>
                                </div>
                                <div class="composer-panel">
                                    <header class="composer-section-header">
                                        <div>
                                            <strong>Labels</strong>
                                            <small>{{ selectedDraftTags.length }} selected</small>
                                        </div>
                                        <button type="button" class="inline-button" @click="tagPickerOpen = !tagPickerOpen">
                                            Labels
                                        </button>
                                    </header>
                                    <div v-if="selectedDraftTags.length" class="composer-tag-list">
                                        <button
                                            v-for="tag in selectedDraftTags"
                                            :key="tag.uuid"
                                            type="button"
                                            class="composer-tag-chip"
                                            @click="removeDraftTag(tag.id)"
                                        >
                                            <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                            {{ tag.name }}
                                            <span aria-hidden="true">&times;</span>
                                        </button>
                                    </div>
                                    <div v-if="tagPickerOpen" class="composer-tag-manager">
                                        <div class="composer-tag-search">
                                            <input
                                                v-model="tagSearchText"
                                                maxlength="255"
                                                placeholder="Search or create label"
                                                @keyup.enter.prevent="createDraftTag"
                                            />
                                            <button type="button" :disabled="tagSaving || !tagSearchText.trim()" @click="createDraftTag">
                                                Create
                                            </button>
                                        </div>
                                        <div v-if="availableDraftTags.length" class="composer-tag-options">
                                            <button
                                                v-for="tag in availableDraftTags"
                                                :key="tag.uuid"
                                                type="button"
                                                class="composer-tag-option"
                                                @click="selectDraftTag(tag)"
                                            >
                                                <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                                {{ tag.name }}
                                            </button>
                                        </div>
                                        <p v-else class="empty-inline">No matching labels</p>
                                    </div>
                                    <div v-if="tagError" class="form-error">{{ tagError }}</div>
                                </div>
                                <div class="composer-panel">
                                    <header class="composer-section-header">
                                        <div>
                                            <strong>Media</strong>
                                            <small>{{ draftPreviewMedia.length }} selected</small>
                                        </div>
                                        <button type="button" class="inline-button" :disabled="mediaSaving" @click="importMediaForDraft">
                                            Import
                                        </button>
                                    </header>
                                    <div v-if="draftPreviewMedia.length" class="composer-selected-media">
                                        <article v-for="item in draftPreviewMedia" :key="item.uuid" class="composer-selected-media-item">
                                            <div class="composer-media-thumb">
                                                <img v-if="mediaThumbnailUrl(item)" :src="mediaAssetUrl(mediaThumbnailUrl(item))" :alt="item.name" />
                                                <span v-else>{{ item.media_type }}</span>
                                            </div>
                                            <div>
                                                <strong>{{ item.name }}</strong>
                                                <small>{{ item.external ? item.source_label : `${item.mime_type} · ${formatBytes(item.size_total)}` }}</small>
                                            </div>
                                            <button
                                                type="button"
                                                class="media-remove-button"
                                                title="Remove media"
                                                :aria-label="`Remove ${item.name}`"
                                                @click="item.external ? removeDraftExternalMedia(item.id) : removeDraftMedia(item.id)"
                                            >
                                                &times;
                                            </button>
                                        </article>
                                    </div>
                                    <div v-if="mediaLibrary.length" class="composer-media-library" aria-label="Media library">
                                        <label
                                            v-for="item in mediaLibrary.slice(0, 12)"
                                            :key="item.uuid"
                                            :class="['composer-media-card', { 'is-selected': isDraftMediaSelected(item) }]"
                                        >
                                            <input
                                                type="checkbox"
                                                :checked="isDraftMediaSelected(item)"
                                                @change="toggleDraftMedia(item)"
                                            />
                                            <span class="composer-media-card-thumb">
                                                <img v-if="mediaThumbnailUrl(item)" :src="mediaAssetUrl(mediaThumbnailUrl(item))" :alt="item.name" />
                                                <span v-else>{{ item.media_type }}</span>
                                            </span>
                                            <span class="composer-media-card-copy">
                                                <strong>{{ item.name }}</strong>
                                                <small>{{ item.media_type }} · {{ formatBytes(item.size_total) }}</small>
                                            </span>
                                        </label>
                                    </div>
                                    <p v-else class="empty-inline">No media in library</p>
                                    <div v-if="mediaError" class="form-error">{{ mediaError }}</div>
                                    <div v-if="mediaProgress" class="form-note">{{ mediaProgress }}</div>
                                </div>
                                <div v-if="draftValidationMessages.length" class="validation-panel is-warning">
                                    <strong>Provider checks</strong>
                                    <ul>
                                        <li v-for="message in draftValidationMessages" :key="message">{{ message }}</li>
                                    </ul>
                                </div>
                                <div class="draft-actions composer-action-bar">
                                    <div class="draft-schedule-control">
                                        <span>
                                            <strong>{{ draftScheduleLabel }}</strong>
                                            <small>{{ draftScheduledAt ? 'Scheduled time' : 'No scheduled time' }}</small>
                                        </span>
                                        <input v-model="draftScheduledAt" class="schedule-input" type="datetime-local" />
                                        <button
                                            v-if="draftScheduledAt"
                                            type="button"
                                            class="inline-button"
                                            :disabled="draftSaving || scheduleSaving"
                                            @click="clearDraftScheduleTime"
                                        >
                                            Clear time
                                        </button>
                                    </div>
                                    <div class="draft-action-buttons">
                                        <span v-if="draftError || scheduleError" class="form-error">{{ draftError || scheduleError }}</span>
                                        <button type="submit" :disabled="draftSaving || !canSaveDraft">{{ draftSubmitLabel }}</button>
                                        <button
                                            v-if="draftScheduledAt"
                                            type="button"
                                            :disabled="draftSaving || scheduleSaving || !canScheduleDraft"
                                            @click="scheduleCurrentDraft()"
                                        >
                                            {{ scheduleSaving ? 'Scheduling' : 'Schedule' }}
                                        </button>
                                        <button
                                            v-else
                                            type="button"
                                            :disabled="draftSaving || scheduleSaving || !canScheduleDraft"
                                            @click="postNowConfirmationOpen = true"
                                        >
                                            Post now
                                        </button>
                                    </div>
                                </div>
                            </form>
                            <div
                                v-if="postNowConfirmationOpen"
                                class="modal-backdrop"
                                role="dialog"
                                aria-modal="true"
                                aria-labelledby="post-now-title"
                                @click.self="postNowConfirmationOpen = false"
                            >
                                <div class="post-now-modal">
                                    <header>
                                        <div>
                                            <h3 id="post-now-title">Confirm publication</h3>
                                            <small>This post will be published immediately.</small>
                                        </div>
                                        <button type="button" class="modal-close-button" aria-label="Close post now confirmation" @click="postNowConfirmationOpen = false">
                                            &times;
                                        </button>
                                    </header>
                                    <p>Publish this post now to the selected social accounts?</p>
                                    <div class="post-now-account-list">
                                        <span v-for="account in selectedDraftAccounts" :key="account.uuid" class="account-chip">
                                            {{ providerDisplayName(account.provider) }} · {{ account.name || account.username || account.provider_id }}
                                        </span>
                                    </div>
                                    <div class="modal-actions">
                                        <button type="button" class="inline-button" @click="postNowConfirmationOpen = false">
                                            Cancel
                                        </button>
                                        <button type="button" :disabled="draftSaving || scheduleSaving || !canScheduleDraft" @click="scheduleCurrentDraft({ postNow: true })">
                                            {{ scheduleSaving ? 'Publishing' : 'Post now' }}
                                        </button>
                                    </div>
                                </div>
                            </div>
                            <div v-if="compactEditorText(draftBody) || draftMediaIds.length || draftExternalMedia.length || selectedDraftAccounts.length" class="provider-preview-grid">
                                <ProviderPreviewCard
                                    v-for="preview in draftPreviewCards"
                                    :key="preview.account.uuid"
                                    :preview="preview"
                                />
                            </div>
                            <div v-if="validationError" class="form-error">{{ validationError }}</div>
                            <div v-if="validationReport" :class="['validation-panel', validationReport.valid ? 'is-ok' : 'is-error']">
                                <strong>{{ validationReport.valid ? 'Ready to schedule' : 'Validation issues' }}</strong>
                                <ul v-if="validationReport.errors.length">
                                    <li v-for="error in validationReport.errors" :key="`${error.account_id}-${error.code}`">
                                        {{ error.message }}
                                    </li>
                                </ul>
                            </div>
                            </div>
                            <div v-show="activePostsMode === 'library'" class="post-library">
                            <div v-if="selectedPostUuids.length" class="selectable-bar">
                                <strong>{{ selectedPostUuids.length }} selected</strong>
                                <div class="row-actions">
                                    <button type="button" class="inline-button" :disabled="draftSaving" @click="clearPostSelection">
                                        Clear
                                    </button>
                                    <button type="button" class="danger-inline-button" :disabled="draftSaving" @click="bulkDeletePosts">
                                        Delete
                                    </button>
                                </div>
                            </div>
                            <div class="post-query-meta">
                                <span>
                                    {{ postPageStart || 0 }}-{{ postPageEnd || 0 }} of {{ postQuery.total }}
                                    post{{ postQuery.total === 1 ? '' : 's' }}
                                </span>
                                <button
                                    v-if="postQuery.items.length"
                                    type="button"
                                    class="inline-button"
                                    :disabled="postQueryLoading"
                                    @click="toggleVisiblePostSelection"
                                >
                                    {{ allVisiblePostsSelected ? 'Clear page' : 'Select page' }}
                                </button>
                            </div>
                            <div v-if="postQueryLoading" class="form-note">Loading posts</div>
                            <div v-if="postQueryError" class="form-error">{{ postQueryError }}</div>
                            <div v-else-if="!postQuery.items.length" class="empty-row">No posts found</div>
                            <div v-else class="post-index-table">
                                <div class="post-index-head">
                                    <label class="post-select">
                                        <input
                                            type="checkbox"
                                            :checked="allVisiblePostsSelected"
                                            @change="toggleVisiblePostSelection"
                                        />
                                    </label>
                                    <span>Status</span>
                                    <span>Content</span>
                                    <span>Media</span>
                                    <span>Labels</span>
                                    <span>Accounts</span>
                                    <span></span>
                                </div>
                                <div v-for="post in postQuery.items" :key="post.uuid" class="post-index-row">
                                    <label class="post-select">
                                        <input v-model="selectedPostUuids" type="checkbox" :value="post.uuid" />
                                    </label>
                                    <div>
                                        <span class="mini-state">{{ post.status }}</span>
                                        <small>{{ post.scheduled_at || post.published_at || post.updated_at }}</small>
                                    </div>
                                    <div class="post-index-content">
                                        <strong>{{ post.preview || post.uuid }}</strong>
                                        <small>{{ post.account_count }} account{{ post.account_count === 1 ? '' : 's' }} · {{ post.tag_count }} label{{ post.tag_count === 1 ? '' : 's' }}</small>
                                        <div v-if="post.status === 'failed' && post.failure_errors?.length" class="failure-list">
                                            <span v-for="message in post.failure_errors.slice(0, 3)" :key="message">
                                                {{ message }}
                                            </span>
                                        </div>
                                    </div>
                                    <div class="post-media-stack">
                                        <div v-for="item in postSummaryMedia(post).slice(0, 3)" :key="item.uuid" class="post-media-thumb">
                                            <img v-if="item.thumb_url || item.url" :src="mediaAssetUrl(item.thumb_url || item.url)" :alt="item.name" />
                                            <span v-else>{{ item.media_type }}</span>
                                        </div>
                                        <span v-if="postSummaryMedia(post).length > 3" class="mini-state">+{{ postSummaryMedia(post).length - 3 }}</span>
                                    </div>
                                    <div class="post-tag-stack">
                                        <span v-for="tag in post.tags" :key="tag.uuid" class="post-tag-chip">
                                            <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                            {{ tag.name }}
                                        </span>
                                    </div>
                                    <div class="post-account-stack">
                                        <span v-for="account in post.accounts.slice(0, 3)" :key="account.uuid" class="account-chip">
                                            {{ account.provider }} · {{ account.username || account.name }}
                                        </span>
                                        <span v-if="post.accounts.length > 3" class="mini-state">+{{ post.accounts.length - 3 }}</span>
                                    </div>
                                    <div class="row-actions">
                                        <button type="button" class="inline-button" :disabled="postDetailLoading" @click="openPostDetail(post)">
                                            View
                                        </button>
                                        <button
                                            v-if="canRetryPost(post)"
                                            type="button"
                                            class="inline-button"
                                            :disabled="scheduleSaving"
                                            @click="retryFailedPostNow(post)"
                                        >
                                            Retry Now
                                        </button>
                                        <button
                                            v-if="canSchedulePost(post) || canRetryPost(post)"
                                            type="button"
                                            class="inline-button"
                                            :disabled="scheduleSaving"
                                            @click="openPostScheduleEditor(post)"
                                        >
                                            {{ post.status === 'failed' ? 'Retry later' : post.scheduled_at ? 'Reschedule' : 'Schedule' }}
                                        </button>
                                        <button v-if="canEditPost(post)" type="button" class="inline-button" :disabled="draftSaving" @click="editPost(post.uuid)">
                                            Edit
                                        </button>
                                        <span v-else class="mini-state is-muted">Locked</span>
                                        <button type="button" class="inline-button" :disabled="draftSaving" @click="duplicatePost(post.uuid)">
                                            Duplicate
                                        </button>
                                        <button type="button" class="inline-button" :disabled="validationRunning" @click="validatePost(post.uuid)">
                                            Validate
                                        </button>
                                        <button type="button" class="danger-inline-button" :disabled="draftSaving" @click="deletePost(post.uuid)">
                                            Delete
                                        </button>
                                    </div>
                                    <ContextualEditor
                                        v-if="editingSchedulePostUuid === post.uuid"
                                        class="post-schedule-editor"
                                        :title="post.status === 'failed' ? 'Retry post later' : 'Schedule post'"
                                        description="Choose when this post should enter the publishing queue."
                                        :save-label="post.status === 'failed' ? 'Schedule retry' : 'Schedule post'"
                                        :busy="scheduleSaving"
                                        @save="schedulePost(post)"
                                        @cancel="closePostScheduleEditor"
                                    >
                                        <label class="contextual-editor-field">
                                            <span>Date and time</span>
                                            <input
                                                v-model="postScheduleDrafts[post.uuid]"
                                                data-contextual-autofocus
                                                class="schedule-input"
                                                type="datetime-local"
                                                :aria-label="post.status === 'failed' ? 'Retry date and time' : 'Schedule date and time'"
                                            />
                                        </label>
                                    </ContextualEditor>
                                </div>
                            </div>
                            <div v-if="!postQueryError && postQuery.total > postQuery.per_page" class="pagination-controls">
                                <span>
                                    {{ postPageStart }}-{{ postPageEnd }} of {{ postQuery.total }}
                                </span>
                                <div class="row-actions">
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="postQueryLoading || postQuery.page <= 1"
                                        @click="changePostPage(postQuery.page - 1)"
                                    >
                                        Previous
                                    </button>
                                    <span class="mini-state">{{ postQuery.page }} / {{ postQuery.total_pages }}</span>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="postQueryLoading || postQuery.page >= postQuery.total_pages"
                                        @click="changePostPage(postQuery.page + 1)"
                                    >
                                        Next
                                    </button>
                                </div>
                            </div>
                            </div>
                        </article>

                        <article v-if="activeView === 'media'" class="snapshot-list">
                            <header>
                                <h3>Media</h3>
                                <div class="row-actions">
                                    <span>{{ mediaLibrary.length }}</span>
                                    <button type="button" class="inline-button" :disabled="mediaSaving" @click="importMultipleMediaFiles">
                                        Import Files
                                    </button>
                                    <button type="button" class="inline-button" :disabled="mediaSaving" @click="cleanupMediaFiles">
                                        Clean
                                    </button>
                                </div>
                            </header>
                            <div class="status-tabs" role="tablist" aria-label="Media source">
                                <button
                                    v-for="tab in mediaTabs"
                                    :key="tab.id"
                                    type="button"
                                    :class="{ 'is-active': activeMediaTab === tab.id }"
                                    role="tab"
                                    :aria-selected="activeMediaTab === tab.id"
                                    @click="setMediaTab(tab.id)"
                                >
                                    {{ tab.label }}
                                </button>
                            </div>
                            <div v-if="localAiLabsEnabled" class="local-ai-panel">
                                <header>
                                    <div>
                                        <strong>Local AI Media Labs</strong>
                                        <small>{{ localAiRuntime.detail }}</small>
                                    </div>
                                    <span :class="['mini-state', localAiRuntime.status === 'ready' ? 'is-ok' : 'is-muted']">
                                        {{ localAiRuntime.accelerator }}
                                    </span>
                                </header>
                                <div class="local-ai-controls">
                                    <button type="button" class="inline-button" :disabled="localAiBusy" @click="probeLocalAiRuntime">
                                        Probe LiteRT
                                    </button>
                                    <select v-model="localAiCropRatio" aria-label="Smart crop ratio">
                                        <option value="square">1:1</option>
                                        <option value="portrait_4_5">4:5</option>
                                        <option value="landscape_16_9">16:9</option>
                                        <option value="story_9_16">9:16</option>
                                    </select>
                                    <form class="local-ai-search-form" @submit.prevent="searchLocalAiMedia">
                                        <input v-model="localAiSearchQuery" placeholder="Semantic media search" />
                                        <button type="submit" :disabled="localAiBusy">Search</button>
                                    </form>
                                </div>
                                <div v-if="localAiBusy" class="form-note local-ai-progress">
                                    <span>{{ localAiResult?.detail || 'Local AI operation running' }}</span>
                                    <button type="button" class="inline-button" @click="cancelLocalAiOperation">Cancel</button>
                                </div>
                                <div v-if="localAiError" class="form-error">{{ localAiError }}</div>
                                <div v-if="localAiResult?.status === 'complete'" class="form-note">
                                    {{ localAiResult.operation }} complete
                                    <span v-if="localAiResult.result?.derivative"> · {{ localAiResult.result.derivative.name }}</span>
                                    <span v-if="localAiResult.result?.alt_text"> · {{ localAiResult.result.alt_text }}</span>
                                </div>
                                <div v-if="localAiResult?.result?.warnings?.length" class="local-ai-warning-list">
                                    <span
                                        v-for="warning in localAiResult.result.warnings"
                                        :key="warning.code"
                                        :class="['mini-state', warning.severity === 'ok' ? 'is-ok' : 'is-muted']"
                                    >
                                        {{ warning.message }}
                                    </span>
                                </div>
                                <div v-if="localAiSearchResults?.matches?.length" class="local-ai-results">
                                    <button
                                        v-for="match in localAiSearchResults.matches.slice(0, 6)"
                                        :key="match.media.uuid"
                                        type="button"
                                        class="local-ai-result"
                                        @click="selectedMediaIds = [match.media.id]"
                                    >
                                        <span>{{ match.media.name }}</span>
                                        <small>{{ match.score }} · {{ match.reasons.join(', ') }}</small>
                                    </button>
                                </div>
                            </div>
                            <div v-if="selectedMediaCount" class="selectable-bar">
                                <strong>{{ selectedMediaCount }} selected</strong>
                                <div class="row-actions">
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="mediaSaving"
                                        :title="selectedExternalMediaPolicyNote"
                                        @click="createPostFromSelectedMedia"
                                    >
                                        Create Post
                                    </button>
                                    <button
                                        v-if="activeMediaTab === 'uploads'"
                                        type="button"
                                        class="danger-inline-button"
                                        :disabled="mediaSaving"
                                        @click="deleteSelectedMedia"
                                    >
                                        Delete
                                    </button>
                                    <button
                                        type="button"
                                        class="inline-button"
                                        :disabled="mediaSaving"
                                        @click="activeMediaTab === 'uploads' ? selectedMediaIds = [] : selectedExternalMediaIds = []"
                                    >
                                        Clear
                                    </button>
                                </div>
                            </div>
                            <div v-if="selectedExternalMediaPolicyNote" class="form-note">{{ selectedExternalMediaPolicyNote }}</div>
                            <form v-if="activeMediaTab === 'uploads'" class="media-filter-form" @submit.prevent="loadMediaLibrary">
                                <input v-model="mediaFilter.keyword" aria-label="Search uploaded media" placeholder="Search media" />
                                <select v-model="mediaFilter.media_type" aria-label="Media type">
                                    <option value="">All</option>
                                    <option value="image">Images</option>
                                    <option value="gif">GIFs</option>
                                    <option value="video">Videos</option>
                                    <option value="file">Files</option>
                                </select>
                                <button type="submit" :disabled="mediaLibraryLoading">Filter</button>
                            </form>
                            <div
                                v-if="activeMediaTab === 'uploads'"
                                :class="['media-drop-zone', { 'is-active': mediaDropActive }]"
                                @dragenter.prevent="mediaDropActive = true"
                                @dragover.prevent="mediaDropActive = true"
                                @dragleave.prevent="mediaDropActive = false"
                                @drop.prevent="handleMediaDrop"
                            >
                                <strong>{{ mediaImport.source_path || 'Drop a local media file' }}</strong>
                                <small>{{ mediaImport.source_path ? 'Ready to import' : 'Images, GIFs, videos, and files' }}</small>
                            </div>
                            <form v-if="activeMediaTab === 'uploads'" class="media-import-form" @submit.prevent="importMediaFile">
                                <input v-model="mediaImport.source_path" aria-label="Local media file path" placeholder="Local file path" />
                                <button type="button" class="inline-button" :disabled="mediaSaving" @click="chooseMediaImportSource">
                                    Choose
                                </button>
                                <input v-model="mediaImport.name" aria-label="Imported media display name" placeholder="Display name" />
                                <button type="submit" :disabled="mediaSaving">Import</button>
                            </form>
                            <form v-if="activeMediaTab === 'uploads'" class="media-download-form" @submit.prevent="downloadExternalMedia">
                                <input v-model="mediaDownload.url" aria-label="Media URL" placeholder="Media URL" />
                                <input v-model="mediaDownload.name" aria-label="Downloaded media display name" placeholder="Display name" />
                                <input v-model="mediaDownload.source" aria-label="Downloaded media source label" placeholder="Source label" />
                                <button type="submit" :disabled="mediaSaving">Download</button>
                            </form>
                            <div v-if="activeMediaTab === 'uploads' && mediaImportResults.length" class="media-import-results">
                                <div
                                    v-for="result in mediaImportResults"
                                    :key="`${result.path}-${result.status}`"
                                    :class="['media-import-result', `is-${result.status}`]"
                                >
                                    <span>{{ result.status }}</span>
                                    <strong>{{ result.name }}</strong>
                                    <small>{{ result.detail }}</small>
                                </div>
                            </div>
                            <form v-if="activeMediaTab !== 'uploads'" class="external-media-form" @submit.prevent="searchExternalMedia(1)">
                                <input
                                    v-model="externalMediaSearch.keyword"
                                    :aria-label="activeMediaTab === 'gifs' ? 'Search GIFs' : 'Search stock media'"
                                    :placeholder="activeMediaTab === 'gifs' ? 'Search KLIPY' : 'Search external media'"
                                />
                                <input v-model="externalMediaSearch.page" min="1" type="number" aria-label="External media page" />
                                <button type="submit" :disabled="externalMediaLoading">Search</button>
                            </form>
                            <div
                                v-if="activeMediaTab !== 'uploads' && !externalMediaResults && !externalMediaLoading && !externalMediaError"
                                class="external-media-empty"
                            >
                                <strong>{{ externalMediaSourceLabel(activeMediaTab) }}</strong>
                                <small>{{ activeMediaTab === 'stock' ? 'Stock image search' : 'GIF search' }}</small>
                            </div>
                            <div v-if="activeMediaTab !== 'uploads' && externalMediaError" class="form-error">{{ externalMediaError }}</div>
                            <div v-if="activeMediaTab !== 'uploads' && externalMediaLoading" class="form-note">Searching external media</div>
                            <div v-if="activeMediaTab !== 'uploads' && externalMediaResults?.items?.length" class="external-media-results">
                                <div v-for="item in externalMediaResults.items" :key="item.id" class="external-media-item">
                                    <label class="post-select">
                                        <input v-model="selectedExternalMediaIds" type="checkbox" :value="item.id" />
                                    </label>
                                    <img :src="mediaAssetUrl(item.thumb_url)" :alt="item.name" />
                                    <div>
                                        <strong>{{ item.name }}</strong>
                                        <small>{{ externalMediaSourceLabel(externalMediaResults.source) }} · {{ item.media_type }}</small>
                                    </div>
                                    <button
                                        v-if="canDownloadExternalMediaItem(item)"
                                        type="button"
                                        :disabled="mediaSaving"
                                        @click="downloadExternalMediaItem(item)"
                                    >
                                        Download
                                    </button>
                                    <small v-else class="provider-policy-note" title="Klipy API terms prohibit permanent content storage except search-result thumbnails.">
                                        Attach only
                                    </small>
                                </div>
                                <button
                                    type="button"
                                    class="inline-button"
                                    :disabled="externalMediaLoading"
                                    @click="searchNextExternalMediaPage"
                                >
                                    Next
                                </button>
                            </div>
                            <div v-if="activeMediaTab !== 'uploads' && externalMediaResults && !externalMediaResults.items.length" class="external-media-empty">
                                <strong>No {{ externalMediaSourceLabel(externalMediaResults.source).toLowerCase() }} results</strong>
                                <small>{{ externalMediaSearch.keyword || `Page ${externalMediaResults.page}` }}</small>
                            </div>
                            <div v-if="mediaError" class="form-error">{{ mediaError }}</div>
                            <div v-if="mediaProgress" class="form-note">{{ mediaProgress }}</div>
                            <div v-if="mediaLibraryError" class="form-error">{{ mediaLibraryError }}</div>
                            <div v-if="activeMediaTab === 'uploads' && mediaLibraryLoading" class="form-note">Loading media</div>
                            <div v-if="activeMediaTab === 'uploads' && mediaCleanup" class="form-note">
                                {{ mediaCleanup.deleted }} files removed · {{ formatBytes(mediaCleanup.reclaimed_bytes) }} reclaimed
                            </div>
                            <div v-if="activeMediaTab === 'uploads' && !mediaLibrary.length" class="empty-row">No media found</div>
                            <div v-for="item in activeMediaTab === 'uploads' ? mediaLibrary : []" :key="item.uuid" class="snapshot-row">
                                <div class="media-row-main">
                                    <label class="post-select">
                                        <input v-model="selectedMediaIds" type="checkbox" :value="item.id" />
                                    </label>
                                    <div class="media-thumb">
                                        <img
                                            v-if="item.thumb_url"
                                            :src="mediaAssetUrl(item.thumb_url)"
                                            :alt="item.name"
                                        />
                                        <span v-else>{{ item.media_type || 'file' }}</span>
                                    </div>
                                    <div>
                                        <strong>{{ item.name }}</strong>
                                        <small>
                                            {{ item.mime_type }} · {{ formatBytes(item.size_total) }} · {{ item.conversion_count }} conversions · {{ item.path }}
                                        </small>
                                    </div>
                                </div>
                                <div class="row-actions">
                                    <span class="mini-state">{{ item.disk }}</span>
                                    <button
                                        v-if="localAiLabsEnabled"
                                        type="button"
                                        class="inline-button"
                                        :disabled="localAiBusy"
                                        @click="preflightLocalAiMedia(item)"
                                    >
                                        Preflight
                                    </button>
                                    <button
                                        v-if="localAiLabsEnabled"
                                        type="button"
                                        class="inline-button"
                                        :disabled="localAiBusy"
                                        @click="draftLocalAiAltText(item)"
                                    >
                                        Alt Text
                                    </button>
                                    <button
                                        v-if="localAiLabsEnabled && localAiStaticImage(item)"
                                        type="button"
                                        class="inline-button"
                                        :disabled="localAiBusy"
                                        @click="upscaleLocalAiMedia(item)"
                                    >
                                        Upscale
                                    </button>
                                    <button
                                        v-if="localAiLabsEnabled && localAiStaticImage(item)"
                                        type="button"
                                        class="inline-button"
                                        :disabled="localAiBusy"
                                        @click="cropLocalAiMedia(item)"
                                    >
                                        Crop
                                    </button>
                                    <button type="button" class="danger-inline-button" :disabled="mediaSaving" @click="deleteMedia(item.uuid)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </article>

                        <article v-if="activeView === 'tags'" class="snapshot-list">
                            <header>
                                    <h3>Labels</h3>
                                <span>{{ snapshot.tags.length }}</span>
                            </header>
                            <form class="tag-form" @submit.prevent="createTag">
                                <input v-model="tagDraft.name" maxlength="255" aria-label="Label name" placeholder="Label name" />
                                <input v-model="tagDraft.hex_color" maxlength="7" aria-label="Label color" placeholder="#101215" />
                                <button type="submit" :disabled="tagSaving">Add</button>
                            </form>
                            <div v-if="tagError" class="form-error">{{ tagError }}</div>
                            <div v-if="!snapshot.tags.length" class="empty-row">No labels yet</div>
                            <div v-for="tag in snapshot.tags" :key="tag.uuid" class="snapshot-row">
                                <ContextualEditor
                                    v-if="editingTagUuid === tag.uuid"
                                    class="tag-edit-form"
                                    title="Edit label"
                                    description="Changes apply anywhere this label is used."
                                    save-label="Save label"
                                    :busy="tagSaving"
                                    @save="updateTag(tag.uuid)"
                                    @cancel="cancelEditTag"
                                >
                                    <label class="contextual-editor-field">
                                        <span>Name</span>
                                        <input
                                            v-model="tagEditDraft.name"
                                            data-contextual-autofocus
                                            maxlength="255"
                                            aria-label="Label name"
                                            placeholder="Label name"
                                        />
                                    </label>
                                    <label class="contextual-editor-field">
                                        <span>Color</span>
                                        <input v-model="tagEditDraft.hex_color" maxlength="7" aria-label="Label color" placeholder="#101215" />
                                    </label>
                                </ContextualEditor>
                                <template v-else>
                                    <div>
                                        <strong>{{ tag.name }}</strong>
                                        <small>{{ tag.post_count }} linked posts</small>
                                    </div>
                                    <div class="row-actions">
                                        <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                        <button type="button" class="inline-button" :disabled="tagSaving" @click="editTag(tag)">
                                            Edit
                                        </button>
                                        <button type="button" class="danger-inline-button" :disabled="tagSaving" @click="deleteTag(tag.uuid)">
                                            Delete
                                        </button>
                                    </div>
                                </template>
                            </div>
                        </article>

                    </div>
                </section>

                <section v-if="activeView === 'settings'" class="panel">
                    <form class="settings-panel-stack" @submit.prevent="saveSettings">
                        <article class="settings-panel">
                            <header>
                                <div>
                                    <h3>Local identity</h3>
                                    <p>The operator information used by this local Dust Wave Social workspace.</p>
                                </div>
                                <span class="mini-state">Local</span>
                            </header>
                            <label class="settings-row">
                                <span>Name</span>
                                <input v-model="settingsDraft.operator_name" maxlength="120" placeholder="Name" />
                            </label>
                            <label class="settings-row">
                                <span>Email</span>
                                <input v-model="settingsDraft.admin_email" type="email" placeholder="Email" />
                            </label>
                            <div class="profile-security-list">
                                <div>
                                    <strong>App Lock</strong>
                                    <small>Dust Wave Social does not provide an app-specific password. Use the macOS user account and device lock to control local access.</small>
                                </div>
                                <div>
                                    <strong>Sign-In Session</strong>
                                    <small>Dust Wave Social runs locally and does not create a server login session.</small>
                                </div>
                            </div>
                        </article>

                        <article class="settings-panel">
                            <header>
                                <div>
                                    <h3>Notifications</h3>
                                    <p>Choose how the desktop app surfaces account and publishing issues.</p>
                                </div>
                                <span class="mini-state">{{ settingsDraft.desktop_notifications ? 'Desktop on' : 'Desktop off' }}</span>
                            </header>
                            <label class="settings-row">
                                <span>Desktop alerts</span>
                                <input v-model="settingsDraft.desktop_notifications" type="checkbox" />
                            </label>
                            <div class="settings-row">
                                <span>Alert identity</span>
                                <small>{{ settingsDraft.admin_email || 'Add an email under Local identity' }}</small>
                            </div>
                            <div class="settings-row">
                                <span>Test alert</span>
                                <button
                                    type="button"
                                    class="inline-button"
                                    :disabled="settingsSaving || !settingsDraft.desktop_notifications"
                                    @click="sendTestNotification"
                                >
                                    Send Test
                                </button>
                            </div>
                            <div v-if="desktopNotificationTestSent" class="form-note">Test notification sent</div>
                            <div v-if="desktopNotificationError" class="form-error">{{ desktopNotificationError }}</div>
                        </article>

                        <article class="settings-panel">
                            <header>
                                <div>
                                    <h3>Labs</h3>
                                    <p>Opt-in local processing tools for media preparation.</p>
                                </div>
                                <span class="mini-state">{{ settingsDraft.local_ai_media_labs ? 'Local AI on' : 'Local AI off' }}</span>
                            </header>
                            <label class="settings-row">
                                <span>Local AI media</span>
                                <input v-model="settingsDraft.local_ai_media_labs" type="checkbox" />
                            </label>
                        </article>

                        <article class="settings-panel">
                            <header>
                                <div>
                                    <h3>Time settings</h3>
                                    <p>Calendar and analytics use these display settings.</p>
                                </div>
                                <span class="mini-state">{{ settingsDraft.timezone || 'Timezone' }}</span>
                            </header>
                            <label class="settings-row">
                                <span>Timezone</span>
                                <select v-model="settingsDraft.timezone" aria-label="Timezone">
                                    <option v-for="timezone in timezoneOptions" :key="timezone" :value="timezone">
                                        {{ timezone }}
                                    </option>
                                </select>
                            </label>
                            <div class="settings-row">
                                <span>Time format</span>
                                <div class="settings-segmented">
                                    <label>
                                        <input v-model="settingsDraft.time_format" type="radio" :value="12" />
                                        12 hour
                                    </label>
                                    <label>
                                        <input v-model="settingsDraft.time_format" type="radio" :value="24" />
                                        24 hour
                                    </label>
                                </div>
                            </div>
                            <div class="settings-row">
                                <span>First day of week</span>
                                <div class="settings-segmented">
                                    <label>
                                        <input v-model="settingsDraft.week_starts_on" type="radio" :value="0" />
                                        Sunday
                                    </label>
                                    <label>
                                        <input v-model="settingsDraft.week_starts_on" type="radio" :value="1" />
                                        Monday
                                    </label>
                                </div>
                            </div>
                            <label class="settings-row">
                                <span>Date display</span>
                                <select v-model="settingsDraft.date_format" aria-label="Date format">
                                    <option value="human">Human date</option>
                                    <option value="iso">ISO date</option>
                                </select>
                            </label>
                        </article>

                        <article class="settings-panel">
                            <header>
                                <div>
                                    <h3>Publishing defaults</h3>
                                    <p>New drafts can start with these accounts selected.</p>
                                </div>
                                <span class="mini-state">{{ settingsDraft.default_accounts.length }} selected</span>
                            </header>
                            <div v-if="snapshot.accounts.length" class="draft-tags">
                                <label v-for="account in snapshot.accounts" :key="account.uuid">
                                    <input v-model="settingsDraft.default_accounts" type="checkbox" :value="account.id" />
                                    {{ account.provider }} · {{ account.username || account.name }}
                                </label>
                            </div>
                            <div v-else class="empty-row">No connected accounts</div>
                        </article>

                        <div v-if="settingsError" class="form-error">{{ settingsError }}</div>
                        <div v-if="settingsSaved" class="form-note">Settings saved</div>
                        <button type="submit" :disabled="settingsSaving">Save Settings</button>
                    </form>
                </section>

            </template>
        </section>
        <PostDetailModal
            v-if="selectedPostSummary"
            :summary="selectedPostSummary"
            :detail="selectedPostDetail"
            :loading="postDetailLoading"
            :error="postDetailError"
            :accounts-count="selectedPostAccounts.length"
            :custom-version-count="selectedPostPreviewCards.filter((preview) => preview.hasCustomVersion).length"
            :tags="selectedPostTags"
            :media-count="selectedPostSummaryMedia.length"
            :media-label="selectedPostSummaryMedia.map((item) => item.media_type).join(', ') || 'None'"
            :preview-cards="selectedPostPreviewCards"
            :timeline="selectedPostTimeline"
            :can-edit="canEditPost(selectedPostSummary)"
            @close="closePostDetail"
            @edit="editSelectedPostFromDetail"
        />
        <ConfirmDialog
            :open="confirmationDialog.open"
            :title="confirmationDialog.title"
            :description="confirmationDialog.description"
            :confirm-label="confirmationDialog.confirmLabel"
            :danger="confirmationDialog.danger"
            @cancel="resolveConfirmation(false)"
            @confirm="resolveConfirmation(true)"
        />
    </main>
</template>
