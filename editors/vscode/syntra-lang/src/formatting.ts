/**
 * @fileoverview Document Formatting Provider for Syntra.
 *
 * Integrates with `syn fmt <file>` to format Syntra source files according to
 * official language canonical conventions.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { execFile } from 'child_process';
import { SyntraLspClient } from './client';

/**
 * Document formatting provider triggered by `Shift+Option+F` or format-on-save.
 *
 * @since v0.1.0
 */
export class SyntraFormattingProvider implements vscode.DocumentFormattingEditProvider {
    private client: SyntraLspClient;

    /**
     * Initializes the formatting provider.
     *
     * @param client - Active LSP client connection.
     */
    constructor(client: SyntraLspClient) {
        this.client = client;
    }

    /**
     * Executes `syn fmt` on the active file and returns text replacement edits.
     *
     * @param document - Document to format.
     * @param _options - Formatting options.
     * @param _token - Cancellation token.
     * @returns Array of text edits replacing document contents with formatted code.
     */
    public provideDocumentFormattingEdits(
        document: vscode.TextDocument,
        _options: vscode.FormattingOptions,
        _token: vscode.CancellationToken
    ): Promise<vscode.TextEdit[]> {
        return new Promise((resolve) => {
            const binary = this.client.resolveSynBinary();
            const filePath = document.uri.fsPath;

            // Execute `syn fmt <file>`
            execFile(binary, ['fmt', filePath], (error: any) => {
                if (error) {
                    vscode.window.showErrorMessage(`Syntra fmt error: ${error.message}`);
                    return resolve([]);
                }

                // Document was formatted in place or text needs reload
                vscode.workspace.fs.readFile(document.uri).then(
                    (bytes) => {
                        const formattedText = Buffer.from(bytes).toString('utf8');
                        const fullRange = new vscode.Range(
                            document.positionAt(0),
                            document.positionAt(document.getText().length)
                        );
                        resolve([vscode.TextEdit.replace(fullRange, formattedText)]);
                    },
                    () => resolve([])
                );
            });
        });
    }
}
