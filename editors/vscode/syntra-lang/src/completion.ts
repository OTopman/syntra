/**
 * @fileoverview Syntra IntelliSense Autocompletion Provider.
 *
 * Provides context-aware completions for language keywords, standard library modules,
 * static member methods, type annotations, and Package.syn manifest configurations.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';
import { isManifestDocument, getManifestCompletions, getDependencyStaticMethods } from './manifest';

/**
 * Autocompletion provider for Syntra source files and package manifests.
 *
 * Combines dynamic LSP server completions with rich client-side static analysis
 * and standard library symbol catalogues.
 *
 * @since v0.1.0
 */
export class SyntraCompletionProvider implements vscode.CompletionItemProvider {
    private client: SyntraLspClient;

    constructor(client: SyntraLspClient) {
        this.client = client;
    }

    public async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
        const linePrefix = document.lineAt(position).text.substring(0, position.character);
        const trimmed = linePrefix.trim();

        // 1. Manifest Document IntelliSense (Package.syn / syntra.toml)
        if (isManifestDocument(document)) {
            const manifestCompletions = getManifestCompletions(document, position, linePrefix);
            if (manifestCompletions.length > 0) {
                return manifestCompletions;
            }
        }

        // 2. Member Access completions triggered by '.'
        if (linePrefix.endsWith('.')) {
            const memberItems = this.getMemberAccessCompletions(linePrefix);
            if (memberItems.length > 0) {
                return memberItems;
            }
        }

        // 3. Type annotation completions triggered by ':'
        if (linePrefix.endsWith(':') || trimmed.endsWith(':') || /:\s*$/.test(linePrefix)) {
            return this.getTypeCompletions();
        }

        // 3. Query LSP server if active
        let lspItems: vscode.CompletionItem[] = [];
        if (this.client.isRunning()) {
            try {
                const res = await this.client.sendRequest('textDocument/completion', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character },
                });

                let rawItems: any[] = [];
                if (Array.isArray(res)) {
                    rawItems = res;
                } else if (res && Array.isArray(res.items)) {
                    rawItems = res.items;
                }

                lspItems = rawItems.map((item) => {
                    let kind = vscode.CompletionItemKind.Property;
                    if (typeof item.kind === 'number') {
                        kind = Math.max(0, item.kind - 1);
                    }
                    const ci = new vscode.CompletionItem(item.label, kind);
                    ci.detail = item.detail;
                    if (item.insertText) {
                        ci.insertText = new vscode.SnippetString(item.insertText);
                    }
                    if (item.documentation) {
                        ci.documentation = new vscode.MarkdownString(
                            typeof item.documentation === 'object'
                                ? item.documentation.value
                                : String(item.documentation)
                        );
                    }
                    return ci;
                });
            } catch (_) {}
        }

        // 4. Merge LSP items with full client-side standard library & syntax knowledge base
        const localItems = this.getAllLanguageCompletions();
        const existingLabels = new Set(lspItems.map((i) => (typeof i.label === 'string' ? i.label : i.label.label)));

        const merged: vscode.CompletionItem[] = [...lspItems];
        for (const item of localItems) {
            const labelStr = typeof item.label === 'string' ? item.label : item.label.label;
            if (!existingLabels.has(labelStr)) {
                merged.push(item);
            }
        }

        return merged;
    }

    private getMemberAccessCompletions(linePrefix: string): vscode.CompletionItem[] {
        const match = linePrefix.match(/([a-zA-Z0-9_]+)\.\s*$/);
        const receiver = match ? match[1] : '';

        // Standard Modules
        if (receiver === 'System') {
            return this.getSystemMembers();
        } else if (receiver === 'Math') {
            return this.getMathMembers();
        } else if (receiver === 'File') {
            return this.getFileMembers();
        } else if (receiver === 'Json') {
            return this.getJsonMembers();
        } else if (receiver === 'Task') {
            return this.getTaskMembers();
        } else if (receiver === 'Channel') {
            return this.getChannelMembers();
        } else if (receiver === 'ThreadPool') {
            return this.getThreadPoolMembers();
        } else if (receiver === 'Dependency') {
            return getDependencyStaticMethods();
        }

        // Generic collection / string methods
        return [
            this.createMethod('length()', 'int', 'Returns the length or element count.', 'length()'),
            this.createMethod('push(item)', 'void', 'Appends an element to the array.', 'push(${1:item})'),
            this.createMethod('pop()', 'item', 'Removes and returns the last element.', 'pop()'),
            this.createMethod('get(key_or_index)', 'value', 'Retrieves value by index or map key.', 'get(${1:index})'),
            this.createMethod('set(key_or_index, value)', 'void', 'Sets value at index or map key.', 'set(${1:key}, ${2:val})'),
            this.createMethod('has(key)', 'bool', 'Checks if key exists in map or set.', 'has(${1:key})'),
            this.createMethod('contains(item)', 'bool', 'Checks if item exists in collection or substring in string.', 'contains(${1:item})'),
            this.createMethod('keys()', 'Array<string>', 'Returns array of map keys.', 'keys()'),
            this.createMethod('values()', 'Array<any>', 'Returns array of map values.', 'values()'),
            this.createMethod('substring(start, end)', 'string', 'Returns substring from start to end index.', 'substring(${1:start}, ${2:end})'),
            this.createMethod('to_upper()', 'string', 'Converts string to uppercase.', 'to_upper()'),
            this.createMethod('to_lower()', 'string', 'Converts string to lowercase.', 'to_lower()'),
            this.createMethod('trim()', 'string', 'Trims whitespace from both ends.', 'trim()'),
            this.createMethod('split(delim)', 'Array<string>', 'Splits string by delimiter.', 'split(${1:delim})'),
            this.createMethod('send(msg)', 'void', 'Sends message through channel.', 'send(${1:msg})'),
            this.createMethod('recv()', 'msg', 'Receives message from channel (blocking/async).', 'recv()'),
            this.createMethod('join()', 'result', 'Awaits task completion and returns result.', 'join()'),
            this.createMethod('is_done()', 'bool', 'Returns true if task execution has finished.', 'is_done()'),
            this.createMethod('cancel()', 'void', 'Signals task cancellation.', 'cancel()'),
        ];
    }

    private getSystemMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('println(msg)', 'void', 'Prints message to standard output with trailing newline.', 'println(${1:\"message\"});'),
            this.createMethod('print(msg)', 'void', 'Prints message to standard output without newline.', 'print(${1:\"message\"});'),
            this.createMethod('args()', 'Array<string>', 'Returns CLI command-line arguments.', 'args()'),
            this.createMethod('exit(code)', 'void', 'Terminates the process with exit code.', 'exit(${1:0});'),
            this.createMethod('time_now_ms()', 'int', 'Returns current Unix epoch time in milliseconds.', 'time_now_ms()'),
            this.createMethod('env(key)', 'string', 'Reads environment variable value.', 'env(${1:\"VAR_NAME\"})'),
        ];
    }

    private getMathMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('sqrt(x)', 'float', 'Computes the square root of x.', 'sqrt(${1:x})'),
            this.createMethod('pow(base, exp)', 'float', 'Computes base raised to power exp.', 'pow(${1:base}, ${2:exp})'),
            this.createMethod('abs(x)', 'float', 'Returns absolute value of x.', 'abs(${1:x})'),
            this.createMethod('min(a, b)', 'number', 'Returns the smaller of two numbers.', 'min(${1:a}, ${2:b})'),
            this.createMethod('max(a, b)', 'number', 'Returns the greater of two numbers.', 'max(${1:a}, ${2:b})'),
            this.createMethod('floor(x)', 'int', 'Rounds down to nearest integer.', 'floor(${1:x})'),
            this.createMethod('ceil(x)', 'int', 'Rounds up to nearest integer.', 'ceil(${1:x})'),
            this.createMethod('sin(x)', 'float', 'Computes sine of angle in radians.', 'sin(${1:x})'),
            this.createMethod('cos(x)', 'float', 'Computes cosine of angle in radians.', 'cos(${1:x})'),
            this.createMethod('tan(x)', 'float', 'Computes tangent of angle in radians.', 'tan(${1:x})'),
            this.createProperty('PI', 'float', 'The mathematical constant π ≈ 3.141592653589793'),
            this.createProperty('E', 'float', 'Euler\'s number e ≈ 2.718281828459045'),
        ];
    }

    private getFileMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('read_to_string(path)', 'string', 'Reads entire file into a UTF-8 string.', 'read_to_string(${1:\"path/to/file\"})'),
            this.createMethod('write_string(path, content)', 'void', 'Writes string content to target file path.', 'write_string(${1:\"path/to/file\"}, ${2:content})'),
            this.createMethod('exists(path)', 'bool', 'Checks if target file exists.', 'exists(${1:\"path/to/file\"})'),
            this.createMethod('delete(path)', 'void', 'Deletes file at path.', 'delete(${1:\"path/to/file\"})'),
        ];
    }

    private getJsonMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('stringify(obj)', 'string', 'Serializes object or array into formatted JSON string.', 'stringify(${1:object})'),
            this.createMethod('parse(str)', 'any', 'Parses JSON string into Syntra Map/Array representation.', 'parse(${1:json_string})'),
        ];
    }

    private getTaskMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('spawn(func)', 'Task<T>', 'Spawns an asynchronous concurrent worker task.', 'spawn(fn() {\n\t$0\n})'),
            this.createMethod('sleep_ms(ms)', 'void', 'Suspends execution for specified milliseconds.', 'sleep_ms(${1:100})'),
        ];
    }

    private getChannelMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('new(capacity)', 'Channel<T>', 'Creates a FIFO channel with optional buffer capacity.', 'new(${1:10})'),
        ];
    }

    private getThreadPoolMembers(): vscode.CompletionItem[] {
        return [
            this.createMethod('new(workers)', 'ThreadPool', 'Creates a bounded worker thread pool.', 'new(${1:4})'),
        ];
    }

    private getTypeCompletions(): vscode.CompletionItem[] {
        const types = [
            { name: 'int', desc: '64-bit signed integer' },
            { name: 'float', desc: '64-bit IEEE-754 floating-point number' },
            { name: 'bool', desc: 'Boolean value (true or false)' },
            { name: 'string', desc: 'UTF-8 string' },
            { name: 'void', desc: 'Unit / void return type' },
            { name: 'Array<T>', desc: 'Generic dynamically sized array', snippet: 'Array<${1:int}>' },
            { name: 'Map<K, V>', desc: 'Generic hash map key-value store', snippet: 'Map<${1:string}, ${2:any}>' },
            { name: 'Task<T>', desc: 'Asynchronous task continuation handle', snippet: 'Task<${1:void}>' },
            { name: 'Channel<T>', desc: 'Synchronization FIFO channel', snippet: 'Channel<${1:string}>' },
            { name: 'ThreadPool', desc: 'Bounded multi-worker concurrency pool' },
        ];

        return types.map((t) => {
            const item = new vscode.CompletionItem(t.name, vscode.CompletionItemKind.TypeParameter);
            item.detail = t.desc;
            if (t.snippet) {
                item.insertText = new vscode.SnippetString(t.snippet);
            }
            return item;
        });
    }

    private getAllLanguageCompletions(): vscode.CompletionItem[] {
        const items: vscode.CompletionItem[] = [];

        // 1. Keywords & Structural Declarations
        items.push(
            this.createSnippet('func', 'Function declaration with typed return', 'func ${1:name}(${2:params}): ${3:void} {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('fn', 'Anonymous lambda closure', 'fn(${1:params}) {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('class', 'Class declaration with constructor', 'class ${1:ClassName} {\n\tpublic ${2:int} ${3:id};\n\n\tnew(${2:int} ${3:id}) {\n\t\tthis.${3:id} = ${3:id};\n\t}\n\n\t$0\n}', vscode.CompletionItemKind.Class),
            this.createSnippet('interface', 'Interface contract declaration', 'interface ${1:InterfaceName} {\n\tfunc ${2:method_name}(${3:params}): ${4:void};\n}', vscode.CompletionItemKind.Interface),
            this.createSnippet('new', 'Instantiate new class instance', 'new ${1:ClassName}(${2:args})', vscode.CompletionItemKind.Keyword),
            this.createSnippet('if', 'Conditional branch', 'if ${1:condition} {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('if else', 'Conditional if-else branch', 'if ${1:condition} {\n\t$1\n} else {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('while', 'While loop', 'while ${1:condition} {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('for in', 'For-in iteration loop', 'for ${1:item} in ${2:collection} {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('for range', 'Numerical range loop', 'for ${1:i} in 0..${2:10} {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('try catch', 'Try-catch exception handling block', 'try {\n\t$1\n} catch (err) {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('select', 'Concurrent channel select block', 'select {\n\tcase ${1:msg} = ${2:channel}.recv():\n\t\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('async func', 'Asynchronous task function', 'async func ${1:name}(): Task<${2:void}> {\n\t$0\n}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('await', 'Await async task completion', 'await ${1:task}', vscode.CompletionItemKind.Keyword),
            this.createSnippet('return', 'Return statement', 'return ${1:value};', vscode.CompletionItemKind.Keyword),
            this.createSnippet('const', 'Constant definition', 'const ${1:int} ${2:NAME} = ${3:value};', vscode.CompletionItemKind.Keyword),
            this.createSnippet('let', 'Variable declaration', 'let ${1:name} = ${2:value};', vscode.CompletionItemKind.Keyword),
            this.createSnippet('import', 'Import module', 'import ${1:module};', vscode.CompletionItemKind.Keyword),
            this.createSnippet('namespace', 'Namespace container block', 'namespace ${1:NamespaceName} {\n\t$0\n}', vscode.CompletionItemKind.Module),
            this.createKeyword('implements', 'Implements interface contract'),
            this.createKeyword('public', 'Public visibility modifier'),
            this.createKeyword('private', 'Private visibility modifier'),
            this.createKeyword('static', 'Static member modifier'),
            this.createKeyword('this', 'Current instance reference'),
            this.createKeyword('throw', 'Throw runtime exception'),
            this.createKeyword('true', 'Boolean literal true'),
            this.createKeyword('false', 'Boolean literal false')
        );

        // 2. Types
        items.push(...this.getTypeCompletions());

        // 3. Built-in Standard Library Modules
        items.push(
            this.createModule('System', 'Operating system, I/O, process, and CLI argument interfaces'),
            this.createModule('Math', 'Mathematical operations, trigonometrics, and constants'),
            this.createModule('File', 'File system read, write, and query APIs'),
            this.createModule('Json', 'JSON serialization and parsing engine'),
            this.createModule('Task', 'Asynchronous concurrent execution task manager'),
            this.createModule('Channel', 'Bounded/unbounded FIFO concurrency channel primitive'),
            this.createModule('ThreadPool', 'Bounded thread pool concurrency executor'),
            this.createModule('Dependency', 'Package manifest dependency constructor module (Package.syn)')
        );

        return items;
    }

    private createMethod(label: string, returnType: string, doc: string, snippet: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Method);
        item.detail = `(method) ${label}: ${returnType}`;
        item.documentation = new vscode.MarkdownString(doc);
        item.insertText = new vscode.SnippetString(snippet);
        return item;
    }

    private createProperty(label: string, type: string, doc: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Property);
        item.detail = `(property) ${label}: ${type}`;
        item.documentation = new vscode.MarkdownString(doc);
        return item;
    }

    private createSnippet(label: string, detail: string, snippet: string, kind: vscode.CompletionItemKind): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, kind);
        item.detail = detail;
        item.insertText = new vscode.SnippetString(snippet);
        return item;
    }

    private createKeyword(label: string, detail: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Keyword);
        item.detail = detail;
        return item;
    }

    private createModule(label: string, detail: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Module);
        item.detail = `module ${label}`;
        item.documentation = new vscode.MarkdownString(detail);
        return item;
    }
}
