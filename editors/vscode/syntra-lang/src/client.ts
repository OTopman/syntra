/**
 * @fileoverview Syntra Language Server Protocol (LSP) Client.
 *
 * Spawns and manages the lifecycle of the background `syn lsp` JSON-RPC process,
 * framing communication via `Content-Length` headers, synchronizing document states,
 * and dispatching diagnostics to VS Code's editor surface.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Client connection manager interfacing VS Code with the native `syn lsp` process.
 *
 * Implements standard JSON-RPC 2.0 over standard I/O streams with robust error recovery,
 * automatic binary path resolution, and live document state synchronization.
 *
 * @example
 * ```typescript
 * const client = new SyntraLspClient(context, diagnosticCollection);
 * client.start();
 * ```
 * @since v0.1.0
 */
export class SyntraLspClient {
    private clientProcess: ChildProcess | null = null;
    private diagnosticCollection: vscode.DiagnosticCollection;
    private messageId = 1;
    private pendingRequests = new Map<number, { resolve: (res: any) => void; reject: (err: any) => void }>();
    private buffer = Buffer.alloc(0);
    private activeUriVersions = new Map<string, number>();
    private onStatusChangeEmitter = new vscode.EventEmitter<boolean>();

    /** Event triggered whenever the LSP server process status transitions between running and stopped. */
    public readonly onStatusChange = this.onStatusChangeEmitter.event;

    /**
     * Initializes a new `SyntraLspClient` instance.
     *
     * @param _context - The extension context.
     * @param diagnosticCollection - Collection instance for publishing editor diagnostics.
     */
    constructor(_context: vscode.ExtensionContext, diagnosticCollection: vscode.DiagnosticCollection) {
        this.diagnosticCollection = diagnosticCollection;
    }

    /**
     * Checks whether the underlying LSP server process is currently running and responsive.
     *
     * @returns `true` if process exists and has not exited.
     */
    public isRunning(): boolean {
        return this.clientProcess !== null && !this.clientProcess.killed;
    }

    /**
     * Resolves the absolute path to the `syn` compiler executable on the host system.
     *
     * Resolution order:
     * 1. User setting `syntra.lsp.path`
     * 2. Workspace `target/debug/syn` or `target/release/syn`
     * 3. System `/usr/local/bin/syn`
     * 4. User cargo binary `~/.cargo/bin/syn`
     * 5. PATH fallback `"syn"`
     *
     * @returns Absolute executable path or command name.
     */
    public resolveSynBinary(): string {
        const config = vscode.workspace.getConfiguration('syntra');
        const configuredPath = config.get<string>('lsp.path');
        if (configuredPath && configuredPath !== 'syn') {
            if (fs.existsSync(configuredPath)) return configuredPath;
        }

        // 1. Check workspace target/debug/syn or target/release/syn
        if (vscode.workspace.workspaceFolders) {
            for (const folder of vscode.workspace.workspaceFolders) {
                const debugBin = path.join(
                    folder.uri.fsPath,
                    'target',
                    'debug',
                    process.platform === 'win32' ? 'syn.exe' : 'syn'
                );
                if (fs.existsSync(debugBin)) return debugBin;

                const releaseBin = path.join(
                    folder.uri.fsPath,
                    'target',
                    'release',
                    process.platform === 'win32' ? 'syn.exe' : 'syn'
                );
                if (fs.existsSync(releaseBin)) return releaseBin;
            }
        }

        // 2. Check /usr/local/bin/syn
        if (fs.existsSync('/usr/local/bin/syn')) return '/usr/local/bin/syn';

        // 3. Check ~/.cargo/bin/syn
        const home = process.env.HOME || process.env.USERPROFILE;
        if (home) {
            const cargoBin = path.join(home, '.cargo', 'bin', process.platform === 'win32' ? 'syn.exe' : 'syn');
            if (fs.existsSync(cargoBin)) return cargoBin;
        }

        // 4. Fallback to configured or 'syn'
        return configuredPath || 'syn';
    }

    /**
     * Spawns the `syn lsp` server child process and establishes the JSON-RPC communication loop.
     */
    public start(): void {
        const config = vscode.workspace.getConfiguration('syntra');
        if (!config.get<boolean>('lsp.enabled', true)) {
            return;
        }

        this.stop();

        const binary = this.resolveSynBinary();
        const cwd = vscode.workspace.workspaceFolders
            ? vscode.workspace.workspaceFolders[0].uri.fsPath
            : undefined;

        try {
            this.clientProcess = spawn(binary, ['lsp'], {
                cwd,
                shell: process.platform === 'win32',
            });
        } catch (err: any) {
            vscode.window.showWarningMessage(
                `Syntra LSP: Failed to start server '${binary} lsp': ${err.message}`
            );
            return;
        }

        this.clientProcess.on('error', (err: any) => {
            vscode.window.showWarningMessage(
                `Syntra LSP: Executable '${binary}' not reachable (${err.message}). Rich syntax highlighting and client-side IntelliSense remain active.`
            );
            this.clientProcess = null;
            this.onStatusChangeEmitter.fire(false);
        });

        this.clientProcess.on('exit', () => {
            this.clientProcess = null;
            this.onStatusChangeEmitter.fire(false);
        });

        this.buffer = Buffer.alloc(0);

        if (this.clientProcess.stdout) {
            this.clientProcess.stdout.on('data', (chunk: Buffer) => {
                this.buffer = Buffer.concat([this.buffer, chunk]);
                while (true) {
                    const headerEnd = this.buffer.indexOf('\r\n\r\n');
                    if (headerEnd === -1) break;

                    const headerStr = this.buffer.slice(0, headerEnd).toString('utf8');
                    let contentLength = 0;
                    for (const line of headerStr.split('\r\n')) {
                        if (line.toLowerCase().startsWith('content-length:')) {
                            contentLength = parseInt(line.split(':')[1].trim(), 10);
                        }
                    }

                    const bodyStart = headerEnd + 4;
                    if (this.buffer.length < bodyStart + contentLength) {
                        break; // Incomplete payload, wait for next chunk
                    }

                    const bodyBuf = this.buffer.slice(bodyStart, bodyStart + contentLength);
                    this.buffer = this.buffer.slice(bodyStart + contentLength);

                    try {
                        const msg = JSON.parse(bodyBuf.toString('utf8'));
                        this.handleRpcMessage(msg);
                    } catch (e) {
                        console.error('Failed to parse LSP payload', e);
                    }
                }
            });
        }

        // Send initialize request
        this.sendRequest('initialize', {
            processId: process.pid,
            rootUri: vscode.workspace.workspaceFolders
                ? vscode.workspace.workspaceFolders[0].uri.toString()
                : null,
            capabilities: {
                textDocument: {
                    hover: { dynamicRegistration: true },
                    completion: { dynamicRegistration: true },
                    definition: { dynamicRegistration: true },
                    documentSymbol: { dynamicRegistration: true },
                },
            },
        })
            .then(() => {
                this.sendNotification('initialized', {});
                this.onStatusChangeEmitter.fire(true);
                // Synchronize all open Syntra documents
                vscode.workspace.textDocuments.forEach((doc) => {
                    if (doc.languageId === 'syntra') {
                        this.syncDocumentOpen(doc);
                    }
                });
            })
            .catch((err) => {
                console.error('LSP initialization error:', err);
            });
    }

    /**
     * Terminates the background LSP process if currently running.
     */
    public stop(): void {
        if (this.clientProcess) {
            try {
                this.clientProcess.kill();
            } catch (_) {}
            this.clientProcess = null;
            this.onStatusChangeEmitter.fire(false);
        }
    }

    /**
     * Sends a JSON-RPC request to the LSP server and awaits the response.
     *
     * @typeParam T - Expected return payload type.
     * @param method - The LSP method string (e.g. `'textDocument/hover'`).
     * @param params - The method parameters object.
     * @returns Promise resolving to the server's response result.
     */
    public sendRequest<T = any>(method: string, params: any): Promise<T> {
        return new Promise((resolve, reject) => {
            if (!this.clientProcess || this.clientProcess.killed || !this.clientProcess.stdin?.writable) {
                return reject(new Error('Syntra LSP server is not running'));
            }

            const id = this.messageId++;
            const payload = JSON.stringify({
                jsonrpc: '2.0',
                id,
                method,
                params,
            });

            const header = `Content-Length: ${Buffer.byteLength(payload, 'utf8')}\r\n\r\n`;
            this.pendingRequests.set(id, { resolve, reject });

            try {
                this.clientProcess.stdin.write(header + payload);
            } catch (err) {
                this.pendingRequests.delete(id);
                reject(err);
            }
        });
    }

    /**
     * Sends a one-way notification to the LSP server without awaiting a response.
     *
     * @param method - The LSP notification method string.
     * @param params - Notification parameters object.
     */
    public sendNotification(method: string, params: any): void {
        if (!this.clientProcess || this.clientProcess.killed || !this.clientProcess.stdin?.writable) {
            return;
        }

        const payload = JSON.stringify({
            jsonrpc: '2.0',
            method,
            params,
        });

        const header = `Content-Length: ${Buffer.byteLength(payload, 'utf8')}\r\n\r\n`;
        try {
            this.clientProcess.stdin.write(header + payload);
        } catch (_) {}
    }

    /**
     * Synchronizes a newly opened document with the language server via `textDocument/didOpen`.
     *
     * @param doc - The VS Code text document.
     */
    public syncDocumentOpen(doc: vscode.TextDocument): void {
        this.activeUriVersions.set(doc.uri.toString(), doc.version);
        this.sendNotification('textDocument/didOpen', {
            textDocument: {
                uri: doc.uri.toString(),
                languageId: 'syntra',
                version: doc.version,
                text: doc.getText(),
            },
        });
    }

    /**
     * Synchronizes live document text edits with the language server via `textDocument/didChange`.
     *
     * @param e - Document change event containing updated buffer.
     */
    public syncDocumentChange(e: vscode.TextDocumentChangeEvent): void {
        const doc = e.document;
        if (doc.languageId !== 'syntra') return;

        this.activeUriVersions.set(doc.uri.toString(), doc.version);
        this.sendNotification('textDocument/didChange', {
            textDocument: {
                uri: doc.uri.toString(),
                version: doc.version,
            },
            contentChanges: [{ text: doc.getText() }],
        });
    }

    /**
     * Notifies the language server that a document was closed via `textDocument/didClose`
     * and clears associated diagnostics.
     *
     * @param doc - The closed text document.
     */
    public syncDocumentClose(doc: vscode.TextDocument): void {
        if (doc.languageId !== 'syntra') return;
        this.activeUriVersions.delete(doc.uri.toString());
        this.diagnosticCollection.delete(doc.uri);
        this.sendNotification('textDocument/didClose', {
            textDocument: { uri: doc.uri.toString() },
        });
    }

    /**
     * Internal handler parsing JSON-RPC messages from the server stdout stream.
     *
     * Resolves pending request promises and publishes compiler diagnostics.
     *
     * @param msg - Parsed JSON-RPC message object.
     */
    private handleRpcMessage(msg: any): void {
        if (msg.id !== undefined && this.pendingRequests.has(msg.id)) {
            const { resolve, reject } = this.pendingRequests.get(msg.id)!;
            this.pendingRequests.delete(msg.id);
            if (msg.error) {
                reject(msg.error);
            } else {
                resolve(msg.result);
            }
            return;
        }

        if (msg.method === 'textDocument/publishDiagnostics') {
            const params = msg.params;
            const uri = vscode.Uri.parse(params.uri);
            const diagnostics: vscode.Diagnostic[] = (params.diagnostics || []).map((d: any) => {
                const range = new vscode.Range(
                    d.range.start.line,
                    d.range.start.character,
                    d.range.end.line,
                    d.range.end.character
                );
                const severity =
                    d.severity === 1
                        ? vscode.DiagnosticSeverity.Error
                        : vscode.DiagnosticSeverity.Warning;
                const diag = new vscode.Diagnostic(range, d.message, severity);
                diag.code = d.code;
                diag.source = d.source || 'syntra';
                return diag;
            });
            this.diagnosticCollection.set(uri, diagnostics);
        }
    }
}
