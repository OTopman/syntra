/**
 * @fileoverview Go to Definition Provider for Syntra.
 *
 * Implements navigation to symbol declarations (classes, interfaces, functions, variables)
 * by delegating to the LSP server or falling back to high-speed regex-based lexical symbol scanning.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';

/**
 * Definition provider enabling Go to Definition (`F12`) across Syntra files.
 *
 * @since v0.1.0
 */
export class SyntraDefinitionProvider implements vscode.DefinitionProvider {
    private client: SyntraLspClient;

    /**
     * Initializes the definition provider.
     *
     * @param client - Active LSP client connection.
     */
    constructor(client: SyntraLspClient) {
        this.client = client;
    }

    /**
     * Resolves the definition location for the symbol under the cursor.
     *
     * @param document - Active text document.
     * @param position - Cursor position.
     * @param _token - Cancellation token.
     * @returns Target definition location or null if unresolved.
     */
    public async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Location | null> {
        // 1. Query LSP server if active
        if (this.client.isRunning()) {
            try {
                const res = await this.client.sendRequest('textDocument/definition', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character },
                });

                if (res && res.range) {
                    const targetUri = vscode.Uri.parse(res.uri);
                    const targetRange = new vscode.Range(
                        res.range.start.line,
                        res.range.start.character,
                        res.range.end.line,
                        res.range.end.character
                    );
                    return new vscode.Location(targetUri, targetRange);
                }
            } catch (_) {}
        }

        // 2. Client-side heuristic symbol lookup
        const wordRange = document.getWordRangeAtPosition(position);
        if (!wordRange) return null;
        const targetWord = document.getText(wordRange);

        const text = document.getText();
        const lines = text.split('\n');

        // Look for declaration patterns: class Name, interface Name, func name, let name, const name
        const patterns = [
            new RegExp(`\\bclass\\s+${targetWord}\\b`),
            new RegExp(`\\binterface\\s+${targetWord}\\b`),
            new RegExp(`\\bfunc\\s+${targetWord}\\b`),
            new RegExp(`\\bconst\\s+[^=]+\\b${targetWord}\\s*=`),
            new RegExp(`\\blet\\s+${targetWord}\\s*=`),
            new RegExp(`\\bnew\\s*\\([^)]*\\)\\s*`),
        ];

        for (let l = 0; l < lines.length; l++) {
            const line = lines[l];
            for (const pat of patterns) {
                const match = line.match(pat);
                if (match && match.index !== undefined) {
                    const col = line.indexOf(targetWord);
                    if (col !== -1) {
                        return new vscode.Location(
                            document.uri,
                            new vscode.Position(l, col)
                        );
                    }
                }
            }
        }

        return null;
    }
}
