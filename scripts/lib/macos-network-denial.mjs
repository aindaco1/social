// Process-scoped test boundary, not a firewall for launchd-owned WKWebView XPCs.
import assert from 'node:assert/strict';
import net from 'node:net';
import dgram from 'node:dgram';
import { randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';

export const NETWORK_DENIAL_PROFILE = '(version 1) (allow default) (deny network*)';
export const CONTROL_ARGUMENT = '--network-controls';
const controlKinds = ['tcp4', 'tcp6', 'udp4', 'udp6'];

export function assertNetworkControls(results, expected) {
    assert.deepEqual(results.map(result => result.kind).sort(), controlKinds, 'All four network controls are required');
    for (const result of results) {
        assert.equal(result.outcome, expected, `${result.kind}: expected ${expected}, got ${JSON.stringify(result)}`);
        if (expected === 'blocked') assert.ok(['EPERM', 'EACCES'].includes(result.code), 'Only explicit permission denial proves blocking');
    }
}

export async function probeNetworkControls(targets) {
    assert.deepEqual(targets.map(target => target.kind).sort(), controlKinds);
    return Promise.all(targets.map(target => new Promise(resolve => {
        let socket;
        let timer;
        let finished = false;
        const finish = (outcome, code) => {
            if (finished) return;
            finished = true;
            clearTimeout(timer);
            if (target.kind.startsWith('tcp')) socket?.destroy();
            else { try { socket?.close(); } catch { /* Socket creation may have been denied. */ } }
            resolve({ kind: target.kind, outcome, ...(code ? { code } : {}) });
        };
        const error = error => finish(['EPERM', 'EACCES'].includes(error.code) ? 'blocked' : 'failed', error.code || 'UNKNOWN');
        timer = setTimeout(() => finish('failed', 'TIMEOUT'), 3000);
        try {
            if (target.kind.startsWith('tcp')) {
                socket = net.createConnection({ host: target.host, port: target.port });
                let response = '';
                socket.on('data', data => {
                    response += data;
                    if (response === target.token) finish('connected');
                    else if (!target.token.startsWith(response)) finish('failed', 'INVALID_RESPONSE');
                });
                socket.on('end', () => finish('failed', 'EARLY_EOF'));
            } else {
                socket = dgram.createSocket(target.kind);
                socket.once('message', data => finish(data.toString() === target.token ? 'connected' : 'failed', data.toString() === target.token ? undefined : 'INVALID_RESPONSE'));
                socket.send(target.token, target.port, target.host, sendError => { if (sendError) error(sendError); });
            }
            socket.on('error', error);
        } catch (caught) { error(caught); }
    })));
}

// Ephemeral loopback controls avoid depending on DNS, Internet availability, or
// a third-party endpoint. The unsandboxed parent must reach them before AND after.
export async function createNetworkControls() {
    const targets = [];
    const servers = [];
    const connections = new Set();
    const close = async () => {
        for (const connection of connections) connection.destroy();
        await Promise.all(servers.map(server => new Promise(resolve => {
            try { server.close(resolve); } catch { resolve(); }
        })));
    };
    try {
        for (const kind of controlKinds) {
            const host = kind.endsWith('4') ? '127.0.0.1' : '::1';
            const token = randomUUID();
            const server = kind.startsWith('tcp') ? net.createServer(connection => {
                connections.add(connection);
                connection.on('error', () => {});
                connection.once('close', () => connections.delete(connection));
                connection.end(token);
            }) : dgram.createSocket(kind);
            servers.push(server);
            if (kind.startsWith('udp')) server.on('message', (data, remote) => {
                if (data.toString() === token) server.send(data, remote.port, remote.address);
            });
            await new Promise((resolve, reject) => {
                server.once('error', reject);
                if (kind.startsWith('tcp')) server.listen({ host, port: 0 }, resolve);
                else server.bind(0, host, resolve);
            });
            targets.push({ kind, host, port: server.address().port, token });
        }
        return { targets, close };
    } catch (error) { await close(); throw error; }
}

export async function runNetworkDenied(command, args) {
    assert.equal(process.platform, 'darwin', 'Network-denied inference requires macOS sandbox-exec');
    const controls = await createNetworkControls();
    try {
        const before = await probeNetworkControls(controls.targets);
        assertNetworkControls(before, 'connected');
        const stdout = await new Promise((resolve, reject) => {
            const child = spawn('/usr/bin/sandbox-exec', ['-p', NETWORK_DENIAL_PROFILE, command, ...args, CONTROL_ARGUMENT, JSON.stringify(controls.targets)], {
                env: {}, stdio: ['ignore', 'pipe', 'pipe'],
            });
            let stdout = '';
            let stderr = '';
            let failure;
            const abort = message => { failure = new Error(message); child.kill('SIGKILL'); };
            const interrupt = () => abort('Network-denied check interrupted');
            process.once('SIGINT', interrupt);
            process.once('SIGTERM', interrupt);
            const timeout = setTimeout(() => abort('Network-denied check timed out after 180s'), 180000);
            child.stdout.on('data', data => {
                stdout += data;
                if (stdout.length > 1024 * 1024) abort('Unexpectedly large test report');
            });
            child.stderr.on('data', data => { stderr = (stderr + data).slice(-16000); });
            child.once('error', error => { failure = error; });
            child.once('close', code => {
                clearTimeout(timeout);
                process.removeListener('SIGINT', interrupt);
                process.removeListener('SIGTERM', interrupt);
                if (failure || code !== 0) reject(failure || new Error(`Sandboxed check failed (${code}): ${stderr || stdout}`));
                else resolve(stdout);
            });
        });
        const after = await probeNetworkControls(controls.targets);
        assertNetworkControls(after, 'connected');
        return { stdout, parent_controls: { before, after } };
    } finally { await controls.close(); }
}
