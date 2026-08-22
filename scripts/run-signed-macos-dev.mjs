#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { signLocalMacosCode } from './lib/macos-local-signing.mjs';

const [executable, ...args] = process.argv.slice(2);

if (!executable) {
    console.error('Cargo did not provide a desktop executable to the macOS development runner.');
    process.exit(1);
}

const childEnv = { ...process.env };

if (process.platform === 'darwin') {
    try {
        const signing = signLocalMacosCode(executable);

        if (signing.stable) {
            childEnv.DUSTWAVE_KEYCHAIN_MODE = 'keychain';
            console.log(`Signed local desktop executable with ${signing.identity}.`);
        } else {
            childEnv.DUSTWAVE_KEYCHAIN_MODE = 'environment';
            console.warn('No Developer ID identity is available; running with environment-only credentials to avoid repeated Keychain prompts.');
        }
    } catch (error) {
        console.error(`Local desktop signing failed; refusing to launch an unstable Keychain requester. ${error.message}`);
        process.exit(1);
    }
}

const child = spawn(executable, args, {
    env: childEnv,
    stdio: 'inherit',
});

for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
    process.on(signal, () => child.kill(signal));
}

child.on('error', (error) => {
    console.error(`Failed to launch the signed desktop executable: ${error.message}`);
    process.exit(1);
});

child.on('exit', (code, signal) => {
    if (signal) {
        process.kill(process.pid, signal);
        return;
    }

    process.exit(code ?? 1);
});
