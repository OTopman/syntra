/**
 * @fileoverview Syntra Status Bar Widget & Action Menu.
 *
 * Renders language server connection health in the VS Code status bar
 * and provides a quick-pick action menu for execution, compilation, and benchmarks.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';

/**
 * Status bar indicator and quick action launcher for Syntra files.
 *
 * @since v0.1.0
 */
export class SyntraStatusBar {
    private item: vscode.StatusBarItem;

    /**
     * Constructs and initializes the Syntra status bar widget.
     *
     * @param context - Extension context.
     * @param client - LSP client instance.
     */
    constructor(context: vscode.ExtensionContext, client: SyntraLspClient) {
        this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
        this.item.command = 'syntra.showMenu';
        context.subscriptions.push(this.item);

        this.updateStatus(client.isRunning());
        client.onStatusChange((running) => this.updateStatus(running));

        context.subscriptions.push(
            vscode.commands.registerCommand('syntra.showMenu', () => this.showQuickMenu())
        );

        context.subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor && editor.document.languageId === 'syntra') {
                    this.item.show();
                } else if (!vscode.window.activeTextEditor || vscode.window.activeTextEditor.document.languageId !== 'syntra') {
                    this.item.hide();
                }
            })
        );

        if (vscode.window.activeTextEditor?.document.languageId === 'syntra') {
            this.item.show();
        }
    }

    /**
     * Updates the status bar icon and label based on language server process state.
     *
     * @param lspActive - Whether LSP server process is alive.
     */
    private updateStatus(lspActive: boolean): void {
        const icon = lspActive ? '$(zap)' : '$(circle-slash)';
        const status = lspActive ? 'LSP: Ready' : 'LSP: Offline';
        this.item.text = `${icon} Syntra (${status})`;
        this.item.tooltip = `Syntra Language Support\nStatus: ${status}\nClick to view Syntra actions.`;
    }

    /**
     * Displays a quick-pick dropdown offering primary Syntra commands.
     */
    private async showQuickMenu(): Promise<void> {
        const options: Array<{ label: string; detail: string; command: string }> = [
            {
                label: '$(play) Run Current File',
                detail: 'Execute Syntra file with interpreter (syn run)',
                command: 'syntra.runFile',
            },
            {
                label: '$(zap) Run with SAR Adaptive Governor',
                detail: 'Execute with dynamic worker scaling and memory pacing (syn run --sar)',
                command: 'syntra.runWithSar',
            },
            {
                label: '$(package) Build Native Standalone Binary',
                detail: 'Compile to native Mach-O/ELF binary via Cranelift (syn build)',
                command: 'syntra.buildNative',
            },
            {
                label: '$(beaker) Run Autonomous Experimentation (SECE)',
                detail: 'Execute sandbox candidate benchmarks and Pareto tuning (syn experiment)',
                command: 'syntra.experiment',
            },
            {
                label: '$(paintcan) Format Document',
                detail: 'Format code according to canonical styling (syn fmt)',
                command: 'syntra.format',
            },
            {
                label: '$(refresh) Restart Language Server',
                detail: 'Restart background LSP server process',
                command: 'syntra.restartLsp',
            },
        ];

        const selected = await vscode.window.showQuickPick(options, {
            placeHolder: 'Select a Syntra command to execute',
        });

        if (selected) {
            vscode.commands.executeCommand(selected.command);
        }
    }
}
