export const dustWaveTikTokBrokerUrl = 'https://dustwave-tiktok-broker.jogo.workers.dev';
export const dustWaveWebsiteUrl = 'https://dustwave.xyz/social/';
export const dustWaveSocialTermsUrl = 'https://dustwave.xyz/social/terms';
export const dustWaveSocialPrivacyUrl = 'https://dustwave.xyz/social/privacy';
export const dustWaveTikTokPlatforms = 'Web and Desktop';
export const dustWaveTikTokDomainVerification = 'Verify dustwave.xyz once with a DNS TXT record; this covers the website, Terms, and Privacy URLs.';
export const dustWaveTikTokProduct = 'Login Kit; add analytics permissions under Scopes because Display API is not a separate product in the current portal.';
export const dustWaveTikTokReviewGate = 'Use Sandbox to record the real authorization and analytics flow before saving or submitting the Production configuration.';

export const providerSetupGuideUrl = 'https://github.com/aindaco1/social/blob/main/docs/PROVIDER_SETUP.md';

export const facebookPageConnectionPermissions = [
    'pages_show_list',
    'pages_read_engagement',
    'read_insights',
];

export const serviceDefinitions = [
    {
        id: 'facebook',
        label: 'Facebook',
        description: 'Store the Meta app credentials used for Facebook Pages, Instagram publishing, comments, and insights.',
        docsUrl: 'https://developers.facebook.com/docs/development/create-an-app/pages-use-case/',
        setupUrl: 'https://developers.facebook.com/apps',
        configurationSecretRef: 'secret://services/facebook',
        accountProvider: 'facebook',
        accountProviderKeys: ['facebook_page', 'instagram'],
        accountActionLabel: 'Connect Facebook or Instagram',
        setupFields: [
            { key: 'redirect', label: 'OAuth callback URL', value: 'http://localhost/callback' },
            { key: 'login', label: 'Login model', value: 'Facebook Login for Business · user access token configuration' },
            { key: 'permissions', label: 'Connection + import permissions', value: facebookPageConnectionPermissions.join(',') },
        ],
        credentials: [
            { field: 'client_id', label: 'App ID', autocomplete: 'off' },
            { field: 'client_secret', label: 'App Secret', autocomplete: 'new-password', secret: true },
        ],
        configuration: [
            {
                field: 'login_configuration_id',
                label: 'Login configuration ID',
                defaultValue: '',
                input: 'text',
                placeholder: 'Meta configuration ID',
            },
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
        label: 'Instagram Local Media',
        description: 'Pair this Mac once so Instagram can fetch an image stored locally while a post publishes. This is a Dust Wave service, not a Cloudflare login.',
        docsUrl: 'https://developers.cloudflare.com/r2/api/workers/workers-api-usage/',
        setupUrl: 'https://dash.cloudflare.com/',
        setupActionLabel: 'Open Cloudflare',
        managed: true,
        configurationSecretRef: 'secret://services/media_staging',
        accountProvider: 'facebook',
        accountProviderKeys: ['instagram'],
        accountActionLabel: 'Connect Instagram account',
        setupFields: [
            { key: 'worker', label: 'Worker', value: 'dustwave-media-staging' },
            { key: 'bucket', label: 'R2 bucket', value: 'dustwave-media-staging' },
            { key: 'retention', label: 'Retention', value: 'Temporary high-entropy URLs with 24 hour default TTL' },
        ],
        credentials: [
            { field: 'client_secret', label: 'Access Token (Advanced)', autocomplete: 'new-password', secret: true },
        ],
        configuration: [
            {
                field: 'base_url',
                label: 'Service URL (Advanced)',
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
        accountProvider: 'twitter',
        accountProviderKeys: ['twitter'],
        accountActionLabel: 'Connect X account',
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
        accountProvider: 'tiktok',
        accountProviderKeys: ['tiktok'],
        accountActionLabel: 'Connect TikTok account',
        setupFields: [
            { key: 'platforms', label: 'App platforms', value: dustWaveTikTokPlatforms },
            { key: 'website', label: 'Official website', value: dustWaveWebsiteUrl },
            { key: 'terms', label: 'Public Terms URL', value: dustWaveSocialTermsUrl },
            { key: 'privacy', label: 'Public Privacy URL', value: dustWaveSocialPrivacyUrl },
            { key: 'verification', label: 'URL verification', value: dustWaveTikTokDomainVerification },
            { key: 'product', label: 'Analytics product', value: dustWaveTikTokProduct },
            { key: 'redirect', label: 'Login Kit Web redirect URI', value: `${dustWaveTikTokBrokerUrl}/api/tiktok/oauth/callback` },
            { key: 'scopes', label: 'MVP analytics scopes', value: 'user.info.basic,user.info.stats,video.list' },
            { key: 'publishing_scopes', label: 'Future publishing scopes', value: 'video.upload,video.publish' },
            { key: 'review', label: 'First review gate', value: dustWaveTikTokReviewGate },
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
        verificationTab: 'stock',
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
        verificationTab: 'gifs',
        setupFields: [
            { key: 'access', label: 'Access path', value: 'Create a Partner Panel app, test with 100 calls/hour, then request production access.' },
            { key: 'attribution', label: 'Attribution', value: 'Dust Wave Social supplies “Search KLIPY” and “Powered by KLIPY” with the official logo; preserve this treatment and the provided content attribution.' },
        ],
        credentials: [
            { field: 'client_id', label: 'API Key', autocomplete: 'off' },
        ],
        configuration: [],
    },
];
