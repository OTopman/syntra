/**
 * @fileoverview Syntra Visual Studio Code / Cursor Extension Entry Point.
 *
 * Bootstraps the Language Server Protocol (LSP) client, registers document lifecycle listeners,
 * initializes IntelliSense completion, hover documentation, signature help, code formatting,
 * go-to-definition, document outline symbols, status bar widgets, and command palette actions.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';
import { SyntraCompletionProvider } from './completion';
import { SyntraHoverProvider } from './hover';
import { SyntraSignatureHelpProvider } from './signatureHelp';
import { SyntraFormattingProvider } from './formatting';
import { SyntraDefinitionProvider } from './definition';
import { SyntraDocumentSymbolProvider } from './symbols';
import { registerCommands } from './commands';
import { SyntraStatusBar } from './statusbar';

/** Singleton active instance of the Syntra LSP client connection. */
let client: SyntraLspClient | null = null;

/**
 * Activates the Syntra Visual Studio Code extension.
 *
 * Invoked automatically by VS Code when any `.syn`, `Package.syn`, or `syntra.toml` document is opened.
 *
 * @param context - Extension context provided by the VS Code runtime.
 */
export function activate(context: vscode.ExtensionContext): void {
    const diagnosticCollection = vscode.languages.createDiagnosticCollection('syntra');
    context.subscriptions.push(diagnosticCollection);

    // Initialize LSP Client
    client = new SyntraLspClient(context, diagnosticCollection);
    client.start();

    // Document Synchronization Events
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((doc) => {
            if (doc.languageId === 'syntra') {
                client?.syncDocumentOpen(doc);
            }
        }),
        vscode.workspace.onDidChangeTextDocument((e) => {
            if (e.document.languageId === 'syntra') {
                client?.syncDocumentChange(e);
            }
        }),
        vscode.workspace.onDidCloseTextDocument((doc) => {
            if (doc.languageId === 'syntra') {
                client?.syncDocumentClose(doc);
            }
        }),
        vscode.workspace.onWillSaveTextDocument((e) => {
            if (e.document.languageId === 'syntra') {
                const config = vscode.workspace.getConfiguration('syntra');
                if (config.get<boolean>('format.onSave', false)) {
                    e.waitUntil(vscode.commands.executeCommand('editor.action.formatDocument'));
                }
            }
        })
    );

    // 1. Completion Provider (IntelliSense)
    const completionProvider = new SyntraCompletionProvider(client);
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            'syntra',
            completionProvider,
            '.',
            ':',
            '(',
            ' ',
            '<',
            '=',
            '"',
            '[',
            '/'
        ),
        vscode.languages.registerCompletionItemProvider('syntra', completionProvider)
    );

    // 2. Hover Provider
    const hoverProvider = new SyntraHoverProvider(client);
    context.subscriptions.push(
        vscode.languages.registerHoverProvider('syntra', hoverProvider)
    );

    // 3. Signature Help Provider
    const signatureHelpProvider = new SyntraSignatureHelpProvider();
    context.subscriptions.push(
        vscode.languages.registerSignatureHelpProvider(
            'syntra',
            signatureHelpProvider,
            '(',
            ','
        )
    );

    // 4. Document Formatting Provider
    const formattingProvider = new SyntraFormattingProvider(client);
    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            'syntra',
            formattingProvider
        )
    );

    // 5. Definition Provider (Go to Definition)
    const definitionProvider = new SyntraDefinitionProvider(client);
    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider('syntra', definitionProvider)
    );

    // 6. Document Symbol Provider (Outline & Breadcrumbs)
    const symbolProvider = new SyntraDocumentSymbolProvider(client);
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider('syntra', symbolProvider)
    );

    // 7. Status Bar Item
    new SyntraStatusBar(context, client);

    // 8. Register Palette Commands
    registerCommands(context, client);
}

/**
 * Deactivates the Syntra extension.
 *
 * Gracefully shuts down the background Language Server child process and disposes allocated resources.
 */
export function deactivate(): void {
    if (client) {
        client.stop();
        client = null;
    }
}
