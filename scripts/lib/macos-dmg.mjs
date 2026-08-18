import { execFile } from 'node:child_process';
import {
    lstat,
    mkdir,
    mkdtemp,
    readdir,
    readlink,
    realpath,
    rm,
} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

export const MACOS_DMG_APP_NAME = 'Dust Wave Social.app';
export const MACOS_DMG_APPLICATIONS_LINK_NAME = 'Applications';
export const MACOS_DMG_APPLICATIONS_LINK_TARGET = '/Applications';
export const MACOS_DMG_FINDER_METADATA_NAME = '.DS_Store';
export const MACOS_DMG_VOLUME_ICON_NAME = '.VolumeIcon.icns';

const REQUIRED_ENTRIES = Object.freeze([
    MACOS_DMG_APP_NAME,
    MACOS_DMG_APPLICATIONS_LINK_NAME,
    MACOS_DMG_VOLUME_ICON_NAME,
].sort());
const ALLOWED_ENTRIES = Object.freeze([
    ...REQUIRED_ENTRIES,
    MACOS_DMG_FINDER_METADATA_NAME,
].sort());
const execFileAsync = promisify(execFile);

async function runTool(executable, args) {
    try {
        return await execFileAsync(executable, args, {
            encoding: 'utf8',
            maxBuffer: 16 * 1024 * 1024,
            timeout: 180_000,
            killSignal: 'SIGKILL',
        });
    } catch (error) {
        const detail = `${error?.stdout || ''}${error?.stderr || ''}`.trim();
        throw new Error(`${executable} ${args.join(' ')} failed${detail ? `:\n${detail}` : ''}`);
    }
}

async function validateDmgPath(dmgInput) {
    if (!dmgInput || !path.isAbsolute(dmgInput)) {
        throw new Error('macOS DMG path must be absolute');
    }

    const dmgPath = path.resolve(dmgInput);
    const dmgStat = await lstat(dmgPath).catch(() => null);

    if (
        dmgPath === path.parse(dmgPath).root
        || path.extname(dmgPath).toLowerCase() !== '.dmg'
        || !dmgStat
        || dmgStat.isSymbolicLink()
        || !dmgStat.isFile()
        || dmgStat.size < 1
    ) {
        throw new Error('macOS DMG must be a non-empty regular .dmg file');
    }

    return { dmgPath, dmgStat };
}

export function applicationsLinkTargetIsValid(target, platform = process.platform) {
    if (target === MACOS_DMG_APPLICATIONS_LINK_TARGET) {
        return true;
    }

    if (platform !== 'win32') {
        return false;
    }

    return /^(?:[A-Za-z]:)?[\\/]Applications$/.test(target);
}

export async function validateMacosDmgLayout(layoutInput) {
    if (!layoutInput || !path.isAbsolute(layoutInput)) {
        throw new Error('macOS DMG layout root must be absolute');
    }

    const layoutRoot = path.resolve(layoutInput);
    const layoutStat = await lstat(layoutRoot).catch(() => null);

    if (
        layoutRoot === path.parse(layoutRoot).root
        || !layoutStat
        || layoutStat.isSymbolicLink()
        || !layoutStat.isDirectory()
    ) {
        throw new Error('macOS DMG layout root is missing or unsafe');
    }

    const entries = (await readdir(layoutRoot)).sort();

    if (
        REQUIRED_ENTRIES.some((entry) => !entries.includes(entry))
        || entries.some((entry) => !ALLOWED_ENTRIES.includes(entry))
    ) {
        throw new Error(`macOS DMG layout entries are invalid: ${entries.join(', ') || 'none'}`);
    }

    const appPath = path.join(layoutRoot, MACOS_DMG_APP_NAME);
    const appStat = await lstat(appPath).catch(() => null);

    if (!appStat || appStat.isSymbolicLink() || !appStat.isDirectory()) {
        throw new Error('macOS DMG app must be a real directory');
    }

    const applicationsPath = path.join(layoutRoot, MACOS_DMG_APPLICATIONS_LINK_NAME);
    const applicationsStat = await lstat(applicationsPath).catch(() => null);

    if (!applicationsStat?.isSymbolicLink()) {
        throw new Error('macOS DMG Applications entry must be a symbolic link');
    }

    const applicationsTarget = await readlink(applicationsPath);

    if (!applicationsLinkTargetIsValid(applicationsTarget)) {
        throw new Error(`macOS DMG Applications link target is invalid: ${applicationsTarget}`);
    }

    const volumeIconPath = path.join(layoutRoot, MACOS_DMG_VOLUME_ICON_NAME);
    const volumeIconStat = await lstat(volumeIconPath).catch(() => null);

    if (!volumeIconStat || volumeIconStat.isSymbolicLink() || !volumeIconStat.isFile() || volumeIconStat.size < 1) {
        throw new Error('macOS DMG volume icon must be a non-empty regular file');
    }

    const finderMetadataPath = path.join(layoutRoot, MACOS_DMG_FINDER_METADATA_NAME);
    const finderMetadataStat = await lstat(finderMetadataPath).catch(() => null);

    if (
        finderMetadataStat
        && (finderMetadataStat.isSymbolicLink() || !finderMetadataStat.isFile() || finderMetadataStat.size < 1)
    ) {
        throw new Error('macOS DMG Finder metadata must be a non-empty regular file');
    }

    return {
        schemaVersion: 'dust-wave-social-macos-dmg-layout-v1',
        appName: MACOS_DMG_APP_NAME,
        applicationsLink: MACOS_DMG_APPLICATIONS_LINK_TARGET,
        entries,
    };
}

async function detach(mountPoint) {
    try {
        await runTool('/usr/bin/hdiutil', ['detach', mountPoint]);
    } catch (error) {
        try {
            await runTool('/usr/bin/hdiutil', ['detach', '-force', mountPoint]);
        } catch {
            throw error;
        }
    }
}

export async function withMountedMacosDmg(dmgInput, callback = null) {
    if (process.platform !== 'darwin') {
        throw new Error('macOS DMG verification can only run on macOS');
    }

    const { dmgPath, dmgStat } = await validateDmgPath(dmgInput);
    const workRoot = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-dmg-verify-'));
    const mountRoot = path.join(workRoot, 'volume');
    await mkdir(mountRoot);
    let attached = false;
    let primaryError;
    let result;

    try {
        await runTool('/usr/bin/hdiutil', ['verify', dmgPath]);
        await runTool('/usr/bin/hdiutil', [
            'attach',
            '-readonly',
            '-nobrowse',
            '-noautoopen',
            '-mountpoint',
            mountRoot,
            dmgPath,
        ]);
        attached = true;

        const canonicalWorkRoot = await realpath(workRoot);
        const canonicalMountRoot = await realpath(mountRoot);
        const containment = path.relative(canonicalWorkRoot, canonicalMountRoot);

        if (!containment || containment === '..' || containment.startsWith(`..${path.sep}`) || path.isAbsolute(containment)) {
            throw new Error('macOS DMG mounted outside its private verification root');
        }

        const layout = await validateMacosDmgLayout(canonicalMountRoot);
        const mounted = {
            dmgPath,
            bytes: dmgStat.size,
            mountPoint: canonicalMountRoot,
            appPath: path.join(canonicalMountRoot, MACOS_DMG_APP_NAME),
            layout,
        };
        const callbackResult = callback ? await callback(mounted) : undefined;

        result = {
            schemaVersion: 'dust-wave-social-macos-dmg-verification-v1',
            bytes: dmgStat.size,
            layout,
            callbackResult,
        };
    } catch (error) {
        primaryError = error;
    } finally {
        if (attached) {
            try {
                await detach(mountRoot);
            } catch (error) {
                primaryError ??= new Error(`failed to detach verified DMG: ${error.message}`);
            }
        }

        try {
            await rm(workRoot, { recursive: true, force: true });
        } catch (error) {
            primaryError ??= new Error(`failed to remove private DMG verification data: ${error.message}`);
        }
    }

    if (primaryError) {
        throw primaryError;
    }

    return result;
}

export async function verifyMacosDmgTrust(dmgInput) {
    if (process.platform !== 'darwin') {
        throw new Error('macOS DMG trust verification can only run on macOS');
    }

    const { dmgPath } = await validateDmgPath(dmgInput);
    await runTool('/usr/bin/codesign', ['--verify', '--strict', '--verbose=2', dmgPath]);
    const details = await runTool('/usr/bin/codesign', ['-dvvv', dmgPath]);
    const detailText = `${details.stdout || ''}${details.stderr || ''}`;

    if (!detailText.includes('Authority=Developer ID Application')) {
        throw new Error('macOS DMG is not signed with a Developer ID Application identity');
    }

    await runTool('/usr/bin/xcrun', ['stapler', 'validate', dmgPath]);
    const assessment = await runTool('/usr/sbin/spctl', [
        '-a',
        '-vv',
        '--type',
        'open',
        '--context',
        'context:primary-signature',
        dmgPath,
    ]);
    const assessmentText = `${assessment.stdout || ''}${assessment.stderr || ''}`;

    if (!/accepted/i.test(assessmentText)) {
        throw new Error(`Gatekeeper did not accept macOS DMG: ${assessmentText.trim()}`);
    }
}
