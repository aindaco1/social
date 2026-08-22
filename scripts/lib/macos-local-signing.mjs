import { spawnSync } from 'node:child_process';

export const MACOS_APP_IDENTIFIER = 'com.dustwave.social';

export function developerIdIdentityFromOutput(output) {
    for (const line of String(output || '').split(/\r?\n/)) {
        const match = line.match(/"(Developer ID Application:[^"]+)"/);

        if (match) {
            return match[1];
        }
    }

    return '';
}

export function resolveDeveloperIdIdentity(env = process.env) {
    const configured = String(env.APPLE_SIGNING_IDENTITY || '').trim();

    if (configured) {
        return configured;
    }

    if (process.platform !== 'darwin') {
        return '';
    }

    const result = spawnSync('/usr/bin/security', ['find-identity', '-p', 'codesigning', '-v'], {
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });

    if (result.status !== 0) {
        return '';
    }

    return developerIdIdentityFromOutput(`${result.stdout || ''}\n${result.stderr || ''}`);
}

export function codesignArguments(target, { deep = false, identity = '' } = {}) {
    return [
        '--force',
        ...(deep ? ['--deep'] : []),
        '--sign',
        identity || '-',
        ...(identity ? ['--identifier', MACOS_APP_IDENTIFIER] : []),
        target,
    ];
}

export function signLocalMacosCode(target, { deep = false, env = process.env } = {}) {
    if (process.platform !== 'darwin') {
        return { identity: '', stable: false, skipped: true };
    }

    const identity = resolveDeveloperIdIdentity(env);
    const sign = spawnSync('/usr/bin/codesign', codesignArguments(target, { deep, identity }), {
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });

    if (sign.status !== 0) {
        throw new Error(sign.stderr || sign.stdout || `Failed to sign ${target}`);
    }

    const verify = spawnSync('/usr/bin/codesign', ['--verify', ...(deep ? ['--deep'] : []), '--strict', '--verbose=2', target], {
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });

    if (verify.status !== 0) {
        throw new Error(verify.stderr || verify.stdout || `Failed to verify ${target}`);
    }

    return { identity, stable: Boolean(identity), skipped: false };
}
