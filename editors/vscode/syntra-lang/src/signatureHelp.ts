/**
 * @fileoverview Syntra Signature Help Provider.
 *
 * Provides real-time parameter hints, method documentation, and active argument position
 * highlighting while typing function or method arguments.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { getManifestSignatureInfo } from './manifest';

/**
 * Signature help provider triggered on `(` and `,`.
 *
 * Resolves standard library function signatures, math operations, system calls,
 * and package manifest DSL functions.
 *
 * @since v0.1.0
 */
export class SyntraSignatureHelpProvider implements vscode.SignatureHelpProvider {
    /**
     * Resolves signature help information for the invocation under the cursor.
     *
     * @param document - Active text document.
     * @param position - Cursor position.
     * @param _token - Cancellation token.
     * @param _context - Signature help context.
     * @returns Populated `vscode.SignatureHelp` or null if not an active invocation.
     */
    public provideSignatureHelp(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken,
        _context: vscode.SignatureHelpContext
    ): vscode.ProviderResult<vscode.SignatureHelp> {
        const lineText = document.lineAt(position.line).text.substring(0, position.character);

        // Find the calling function name before '('
        const match = lineText.match(/([a-zA-Z0-9_.]+)\s*\(([^)]*)$/);
        if (!match) return null;

        const callee = match[1];
        const argsStr = match[2];
        const activeParameter = (argsStr.match(/,/g) || []).length;

        let sigInfo: vscode.SignatureInformation | null = SIGNATURE_REGISTRY[callee] || null;
        if (!sigInfo && callee.startsWith('Dependency.')) {
            sigInfo = getManifestSignatureInfo(callee);
        }
        if (!sigInfo) return null;

        const help = new vscode.SignatureHelp();
        help.signatures = [sigInfo];
        help.activeSignature = 0;
        help.activeParameter = Math.min(activeParameter, sigInfo.parameters.length - 1);

        return help;
    }
}

/** Pre-indexed parameter documentation table for standard library calls. */
const SIGNATURE_REGISTRY: Record<string, vscode.SignatureInformation> = {
    'System.println': createSig('System.println(msg: string): void', 'Prints string to stdout with newline', [
        { label: 'msg: string', doc: 'The string message to display' }
    ]),
    'System.print': createSig('System.print(msg: string): void', 'Prints string to stdout without newline', [
        { label: 'msg: string', doc: 'The string message to display' }
    ]),
    'System.exit': createSig('System.exit(code: int): void', 'Terminates process with return code', [
        { label: 'code: int', doc: 'Standard exit code (0 for success)' }
    ]),
    'Math.sqrt': createSig('Math.sqrt(x: float): float', 'Computes the square root of x', [
        { label: 'x: float', doc: 'Non-negative number' }
    ]),
    'Math.pow': createSig('Math.pow(base: float, exp: float): float', 'Computes base raised to exponent exp', [
        { label: 'base: float', doc: 'The base number' },
        { label: 'exp: float', doc: 'The exponent' }
    ]),
    'Math.min': createSig('Math.min(a: number, b: number): number', 'Returns smaller of two numbers', [
        { label: 'a: number', doc: 'First number' },
        { label: 'b: number', doc: 'Second number' }
    ]),
    'Math.max': createSig('Math.max(a: number, b: number): number', 'Returns greater of two numbers', [
        { label: 'a: number', doc: 'First number' },
        { label: 'b: number', doc: 'Second number' }
    ]),
    'File.readText': createSig('File.readText(path: string): string', 'Reads entire file into a UTF-8 string', [
        { label: 'path: string', doc: 'Path to the target file' }
    ]),
    'File.writeText': createSig('File.writeText(path: string, content: string): bool', 'Overwrites file with string content', [
        { label: 'path: string', doc: 'Path to destination file' },
        { label: 'content: string', doc: 'Text content to write' }
    ]),
    'File.exists': createSig('File.exists(path: string): bool', 'Checks whether a file or directory exists', [
        { label: 'path: string', doc: 'Path to verify' }
    ]),
    'Json.parse': createSig('Json.parse(jsonStr: string): any', 'Parses a JSON string into an object or array', [
        { label: 'jsonStr: string', doc: 'Valid JSON string' }
    ]),
    'Json.stringify': createSig('Json.stringify(value: any): string', 'Serializes a value to formatted JSON', [
        { label: 'value: any', doc: 'Value to serialize' }
    ]),
    'Sha256.hash': createSig('Sha256.hash(input: string): string', 'Computes 64-character lowercase SHA-256 digest', [
        { label: 'input: string', doc: 'Input payload string' }
    ]),
    'HmacSha256.compute': createSig('HmacSha256.compute(key: string, message: string): string', 'Computes HMAC-SHA256 signature', [
        { label: 'key: string', doc: 'Secret cryptographic key' },
        { label: 'message: string', doc: 'Data to authenticate' }
    ]),
};

/**
 * Helper to construct a `vscode.SignatureInformation` object.
 *
 * @param label - Signature label string.
 * @param doc - Method documentation.
 * @param params - Parameter definitions.
 * @returns Configured `SignatureInformation` instance.
 */
function createSig(
    label: string,
    doc: string,
    params: Array<{ label: string; doc: string }>
): vscode.SignatureInformation {
    const sig = new vscode.SignatureInformation(label, new vscode.MarkdownString(doc));
    sig.parameters = params.map(
        (p) => new vscode.ParameterInformation(p.label, new vscode.MarkdownString(p.doc))
    );
    return sig;
}
