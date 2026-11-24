/**
 * @fileoverview Syntra Document Symbol Provider (Breadcrumbs & Outline).
 *
 * Extracts document symbols (namespaces, classes, interfaces, methods, constructors, fields, functions)
 * to populate the editor's Breadcrumbs bar, Quick Outline (`Cmd+Shift+O`), and Outline View.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';
import { isManifestDocument, getManifestSymbols } from './manifest';

/**
 * Document symbol provider for Syntra source code and `Package.syn` manifests.
 *
 * Queries the native language server when available, or executes fast client-side
 * regex extraction to guarantee instant outlines even without an active compiler binary.
 *
 * @since v0.1.0
 */
export class SyntraDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
    private client: SyntraLspClient;

    /**
     * Constructs the document symbol provider.
     *
     * @param client - Active LSP client instance.
     */
    constructor(client: SyntraLspClient) {
        this.client = client;
    }

    /**
     * Extracts hierarchical symbols from the document.
     *
     * @param document - Active text document.
     * @param _token - Cancellation token.
     * @returns Array of hierarchical document symbols.
     */
    public async provideDocumentSymbols(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken
    ): Promise<vscode.DocumentSymbol[]> {
        // 0. Dedicated symbols parser for Package.syn manifest
        if (isManifestDocument(document)) {
            return getManifestSymbols(document);
        }

        // 1. Query LSP server if active
        if (this.client.isRunning()) {
            try {
                const res = await this.client.sendRequest('textDocument/documentSymbol', {
                    textDocument: { uri: document.uri.toString() },
                });

                if (Array.isArray(res) && res.length > 0) {
                    return res.map((sym: any) => {
                        const range = new vscode.Range(
                            sym.range.start.line,
                            sym.range.start.character,
                            sym.range.end.line,
                            sym.range.end.character
                        );
                        const selectionRange = new vscode.Range(
                            sym.selectionRange.start.line,
                            sym.selectionRange.start.character,
                            sym.selectionRange.end.line,
                            sym.selectionRange.end.character
                        );
                        return new vscode.DocumentSymbol(
                            sym.name,
                            sym.detail || '',
                            sym.kind - 1,
                            range,
                            selectionRange
                        );
                    });
                }
            } catch (_) {}
        }

        // 2. Client-side heuristic outline generator
        const symbols: vscode.DocumentSymbol[] = [];
        const text = document.getText();
        const lines = text.split('\n');

        let currentClass: vscode.DocumentSymbol | null = null;

        for (let l = 0; l < lines.length; l++) {
            const line = lines[l];
            const trimmed = line.trim();

            // Classes: class Name ...
            const classMatch = trimmed.match(/^class\s+([a-zA-Z0-9_]+)/);
            if (classMatch) {
                const name = classMatch[1];
                const range = new vscode.Range(l, 0, l, line.length);
                const sym = new vscode.DocumentSymbol(
                    name,
                    'class',
                    vscode.SymbolKind.Class,
                    range,
                    range
                );
                symbols.push(sym);
                currentClass = sym;
                continue;
            }

            // Interfaces: interface Name ...
            const ifaceMatch = trimmed.match(/^interface\s+([a-zA-Z0-9_]+)/);
            if (ifaceMatch) {
                const name = ifaceMatch[1];
                const range = new vscode.Range(l, 0, l, line.length);
                const sym = new vscode.DocumentSymbol(
                    name,
                    'interface',
                    vscode.SymbolKind.Interface,
                    range,
                    range
                );
                symbols.push(sym);
                currentClass = sym;
                continue;
            }

            // Namespaces: namespace Name ...
            const nsMatch = trimmed.match(/^namespace\s+([a-zA-Z0-9_]+)/);
            if (nsMatch) {
                const name = nsMatch[1];
                const range = new vscode.Range(l, 0, l, line.length);
                const sym = new vscode.DocumentSymbol(
                    name,
                    'namespace',
                    vscode.SymbolKind.Namespace,
                    range,
                    range
                );
                symbols.push(sym);
                continue;
            }

            // Functions / Methods: func name(...) or fn name(...)
            const fnMatch = trimmed.match(/(?:public\s+|private\s+|static\s+)*func\s+([a-zA-Z0-9_]+)\s*\(/);
            if (fnMatch) {
                const name = fnMatch[1];
                const range = new vscode.Range(l, 0, l, line.length);
                const kind = currentClass ? vscode.SymbolKind.Method : vscode.SymbolKind.Function;
                const sym = new vscode.DocumentSymbol(name, '()', kind, range, range);
                if (currentClass) {
                    currentClass.children.push(sym);
                } else {
                    symbols.push(sym);
                }
                continue;
            }

            // Constructor: new(...) {
            const ctorMatch = trimmed.match(/^new\s*\(/);
            if (ctorMatch && currentClass) {
                const range = new vscode.Range(l, 0, l, line.length);
                const sym = new vscode.DocumentSymbol('constructor', 'new()', vscode.SymbolKind.Constructor, range, range);
                currentClass.children.push(sym);
                continue;
            }

            // End of class body
            if (trimmed === '}' && currentClass) {
                currentClass.range = new vscode.Range(currentClass.range.start.line, 0, l, line.length);
                currentClass = null;
            }
        }

        return symbols;
    }
}
