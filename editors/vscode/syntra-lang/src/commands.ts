/**
 * @fileoverview Syntra Command Palette Actions & Terminal Execution.
 *
 * Registers interactive commands for running programs, compiling native binaries,
 * triggering SAR adaptation, executing learning and experimentation benchmarks,
 * and formatting source files.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';

/**
 * Registers all Syntra command palette actions and terminal integrations.
 *
 * Available Commands:
 * - `syntra.runFile`: Fast-execute current file with interpreter (`syn run`)
 * - `syntra.runWithSar`: Execute with Self-Adapting Runtime dynamic governor (`syn run --sar`)
 * - `syntra.buildNative`: Compile to standalone native executable binary (`syn build`)
 * - `syntra.experiment`: Run autonomous SECE optimization trial (`syn experiment`)
 * - `syntra.format`: Format active file (`syn fmt`)
 * - `syntra.restartLsp`: Restart language server background process
 *
 * @param context - Extension activation context.
 * @param client - LSP client instance.
 * @since v0.1.0
 */
export function registerCommands(context: vscode.ExtensionContext, client: SyntraLspClient): void {
    let terminal: vscode.Terminal | null = null;

    const getTerminal = (): vscode.Terminal => {
        if (!terminal || terminal.exitStatus !== undefined) {
            terminal = vscode.window.createTerminal('Syntra');
        }
        return terminal;
    };

    const getActiveSynFile = (): string | null => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || (editor.document.languageId !== 'syntra' && !editor.document.fileName.endsWith('.syn'))) {
            vscode.window.showWarningMessage('Please open a Syntra (.syn) file to execute this command.');
            return null;
        }
        return editor.document.fileName;
    };

    // 1. Run File
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.runFile', () => {
            const file = getActiveSynFile();
            if (!file) return;

            const binary = client.resolveSynBinary();
            const term = getTerminal();
            term.show();
            term.sendText(`${binary} run "${file}"`);
        })
    );

    // 2. Run with SAR Governor
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.runWithSar', () => {
            const file = getActiveSynFile();
            if (!file) return;

            const binary = client.resolveSynBinary();
            const term = getTerminal();
            term.show();
            term.sendText(`${binary} run "${file}" --sar`);
        })
    );

    // 3. Build Native Standalone Binary
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.buildNative', () => {
            const file = getActiveSynFile();
            if (!file) return;

            const binary = client.resolveSynBinary();
            const term = getTerminal();
            term.show();
            term.sendText(`${binary} build "${file}"`);
        })
    );

    // 4. Autonomous Experimentation (SECE)
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.experiment', () => {
            const file = getActiveSynFile();
            if (!file) return;

            const binary = client.resolveSynBinary();
            const term = getTerminal();
            term.show();
            term.sendText(`${binary} experiment "${file}" --iterations 10`);
        })
    );

    // 5. Format Document
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.format', () => {
            vscode.commands.executeCommand('editor.action.formatDocument');
        })
    );

    // 6. Restart LSP Server
    context.subscriptions.push(
        vscode.commands.registerCommand('syntra.restartLsp', () => {
            client.start();
            vscode.window.showInformationMessage('Syntra Language Server restarted.');
        })
    );
}
