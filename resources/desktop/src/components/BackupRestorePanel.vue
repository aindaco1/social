<script setup>
defineProps({
    backupRunning: {
        type: Boolean,
        default: false,
    },
    backupError: {
        type: String,
        default: '',
    },
    backupSummary: {
        type: Object,
        default: null,
    },
    restoreRunning: {
        type: Boolean,
        default: false,
    },
    restoreError: {
        type: String,
        default: '',
    },
    restoreSummary: {
        type: Object,
        default: null,
    },
    restorePath: {
        type: String,
        default: '',
    },
});

defineEmits(['backup', 'choose-restore', 'restore', 'update:restorePath']);

const formatBytes = (value) => {
    const bytes = Number(value) || 0;

    if (bytes < 1024) {
        return `${bytes} B`;
    }

    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }

    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
};
</script>

<template>
    <section class="panel">
        <div class="panel-heading">
            <div>
                <h2>Backup And Restore</h2>
                <p>Protect local posts, settings, logs, and app-owned media. OS keychain secrets are not included.</p>
            </div>
            <div class="health-heading-actions">
                <button type="button" class="inline-button" :disabled="backupRunning || restoreRunning" @click="$emit('backup')">
                    Create Backup
                </button>
            </div>
        </div>
        <div class="backup-restore-grid">
            <article class="settings-panel">
                <header>
                    <div>
                        <h3>Latest backup</h3>
                        <p>Backups are written to the app-data backups folder.</p>
                    </div>
                    <span class="mini-state">{{ backupSummary ? formatBytes(backupSummary.bytes) : 'Ready' }}</span>
                </header>
                <div v-if="backupSummary" class="system-detail-grid">
                    <div class="system-detail-row">
                        <span>Folder</span>
                        <strong>{{ backupSummary.path }}</strong>
                    </div>
                    <div class="system-detail-row">
                        <span>Media files</span>
                        <strong>{{ backupSummary.media_files }}</strong>
                    </div>
                    <div class="system-detail-row">
                        <span>Manifest</span>
                        <strong>{{ backupSummary.manifest_path }}</strong>
                    </div>
                </div>
                <div v-else class="empty-row">No backup created this session</div>
                <div v-if="backupError" class="form-error">{{ backupError }}</div>
            </article>
            <article class="settings-panel">
                <header>
                    <div>
                        <h3>Restore backup</h3>
                        <p>Current local data is automatically backed up before restore.</p>
                    </div>
                    <span class="mini-state">Folder</span>
                </header>
                <div class="restore-path-row">
                    <input
                        :value="restorePath"
                        type="text"
                        placeholder="Backup folder path"
                        @input="$emit('update:restorePath', $event.target.value)"
                    />
                    <button type="button" class="inline-button" :disabled="restoreRunning || backupRunning" @click="$emit('choose-restore')">
                        Choose
                    </button>
                    <button type="button" class="danger-inline-button" :disabled="restoreRunning || backupRunning || !restorePath.trim()" @click="$emit('restore')">
                        Restore
                    </button>
                </div>
                <div v-if="restoreSummary" class="system-detail-grid">
                    <div class="system-detail-row">
                        <span>Restored from</span>
                        <strong>{{ restoreSummary.backup_path }}</strong>
                    </div>
                    <div class="system-detail-row">
                        <span>Safety backup</span>
                        <strong>{{ restoreSummary.safety_backup_path }}</strong>
                    </div>
                    <div class="system-detail-row">
                        <span>Restored media</span>
                        <strong>{{ restoreSummary.restored_media_files }} files · {{ formatBytes(restoreSummary.restored_bytes) }}</strong>
                    </div>
                </div>
                <div v-if="restoreError" class="form-error">{{ restoreError }}</div>
            </article>
        </div>
    </section>
</template>
