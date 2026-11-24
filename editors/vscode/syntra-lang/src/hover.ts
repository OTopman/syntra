/**
 * @fileoverview Syntra Rich Hover Information Provider.
 *
 * Provides rich hover tooltips with code examples, parameter details, return types,
 * and standard library reference manuals.
 *
 * @author Syntra Language Engineering Team
 * @version 0.1.0
 */

import * as vscode from 'vscode';
import { SyntraLspClient } from './client';
import { isManifestDocument, getManifestHoverInfo } from './manifest';

/**
 * Hover provider delivering markdown documentation tooltips on hover.
 *
 * Resolves hovers via LSP server, falling back to an embedded comprehensive documentation dictionary.
 *
 * @since v0.1.0
 */
export class SyntraHoverProvider implements vscode.HoverProvider {
    private client: SyntraLspClient;

    constructor(client: SyntraLspClient) {
        this.client = client;
    }

    public async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Hover | null> {
        // 1. Try LSP server first
        if (this.client.isRunning()) {
            try {
                const res = await this.client.sendRequest('textDocument/hover', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character },
                });
                if (res && res.contents) {
                    return new vscode.Hover(new vscode.MarkdownString(res.contents.value));
                }
            } catch (_) {}
        }

        // 2. Client-side fallback hover dictionary
        const wordRange = document.getWordRangeAtPosition(position);
        if (!wordRange) return null;
        const word = document.getText(wordRange);

        const hoverText = this.lookupHoverInfo(word, document, position);
        if (hoverText) {
            return new vscode.Hover(new vscode.MarkdownString(hoverText));
        }

        return null;
    }

    private lookupHoverInfo(word: string, document: vscode.TextDocument, position: vscode.Position): string | null {
        const lineText = document.lineAt(position.line).text;

        // Check Package.syn manifest keywords & Dependency methods
        if (isManifestDocument(document) || word === 'Dependency' || lineText.includes('Dependency.')) {
            const manifestDoc = getManifestHoverInfo(word, lineText);
            if (manifestDoc) {
                return manifestDoc;
            }
        }

        // Check compound methods like System.println, Math.sqrt, etc.
        const qualifiedMatch = lineText.match(/([A-Z][a-zA-Z0-9_]*)\.([a-z0-9_]+)/);
        if (qualifiedMatch) {
            const qualified = `${qualifiedMatch[1]}.${qualifiedMatch[2]}`;
            if (HOVER_DICTIONARY[qualified]) {
                return HOVER_DICTIONARY[qualified];
            }
        }

        return HOVER_DICTIONARY[word] || null;
    }
}

const HOVER_DICTIONARY: Record<string, string> = {
    // --- Core Keywords ---
    'func': '```syntra\nfunc <name>(<params>): <type>\n```\nDeclares a named function with static parameter types and explicit return type.\n\n```syntra\nfunc add(int a, int b): int {\n    return a + b;\n}\n```\n\n@since v0.1.0',
    'fn': '```syntra\nfn(<params>) { ... }\n```\nAnonymous lambda closure expression capturing surrounding lexical scope variables.\n\n```syntra\nlet multiplier = 2;\nlet double = fn(int x) {\n    return x * multiplier;\n};\n```\n\n@since v0.1.0',
    'class': '```syntra\nclass <Name> [implements <Interface>] { ... }\n```\nDefines an object-oriented class with typed fields, constructor (`new`), and instance methods.\n\n```syntra\nclass User {\n    public int id;\n    public string name;\n\n    new(int id, string name) {\n        this.id = id;\n        this.name = name;\n    }\n}\n```\n\n@since v0.1.0',
    'interface': '```syntra\ninterface <Name> { ... }\n```\nDefines an abstract polymorphic contract that implementing classes must satisfy.\n\n```syntra\ninterface Serializable {\n    func serialize(): string;\n}\n```\n\n@since v0.1.0',
    'implements': 'Specifies that a class adheres to the structural contract of an interface.\n\n@since v0.1.0',
    'new': '```syntra\nnew <ClassName>(<args>)\n```\nInstantiates an object, allocating memory and invoking its `new(...)` constructor.\n\n@since v0.1.0',
    'this': 'Refers to the current class instance within instance methods and constructors.\n\n@since v0.1.0',
    'public': 'Visibility modifier: member is accessible from any scope.\n\n@since v0.1.0',
    'private': 'Visibility modifier: member is encapsulated and accessible only within its declaring class.\n\n@since v0.1.0',
    'static': 'Class-level member not bound to an individual object instance.\n\n@since v0.1.0',
    'const': '```syntra\nconst <type> <NAME> = <value>;\n```\nDeclares an immutable constant value evaluated at compile time.\n\n@since v0.1.0',
    'let': '```syntra\nlet <name> = <value>;\n```\nDeclares a local variable with static type inference.\n\n@since v0.1.0',
    'async': 'Marks a function as asynchronous, executing concurrently and returning a `Task<T>` promise handle.\n\n@since v0.1.0',
    'await': 'Suspends execution of the current fiber until an asynchronous `Task<T>` completes and yields its result.\n\n@since v0.1.0',
    'select': 'Multiplexes over concurrent channel operations without busy-waiting.\n\n@since v0.1.0',

    // --- System Module ---
    'System': '```syntra\nnamespace System\n```\nFoundational operating system interfaces for console I/O, process control, environment inspection, and execution timing.\n\n```syntra\nimport System;\n\nfunc main(): void {\n    System.println("Hello from Syntra " + System.timeMs());\n}\n```\n\n@since v0.1.0',
    'System.println': '```syntra\nSystem.println(string msg): void\n```\nPrints string to standard output (`stdout`) followed by an operating system newline.\n\n```syntra\nSystem.println("Execution finished cleanly.");\n```\n\n@param msg The string payload to display on stdout.\n@return void\n@since v0.1.0',
    'System.print': '```syntra\nSystem.print(string msg): void\n```\nPrints string to standard output (`stdout`) without appending a trailing newline.\n\n```syntra\nSystem.print("Enter password: ");\n```\n\n@param msg The string payload to display on stdout.\n@return void\n@since v0.1.0',
    'System.exit': '```syntra\nSystem.exit(int code): void\n```\nTerminates the calling process immediately with the specified OS exit code.\n\n```syntra\nif (hasError) {\n    System.exit(1);\n}\n```\n\n@param code Operating system process return code (0 for success, >0 for errors).\n@return void\n@since v0.1.0',
    'System.args': '```syntra\nSystem.args(): Array<string>\n```\nRetrieves command-line arguments passed to the program upon invocation.\n\n```syntra\nlet args = System.args();\nfor arg in args {\n    System.println("Arg: " + arg);\n}\n```\n\n@return An array of strings representing command-line arguments.\n@since v0.1.0',
    'System.timeMs': '```syntra\nSystem.timeMs(): int\n```\nReturns the current Unix epoch time in milliseconds.\n\n```syntra\nlet start = System.timeMs();\ndoWork();\nlet elapsed = System.timeMs() - start;\n```\n\n@return 64-bit integer timestamp in milliseconds.\n@since v0.1.0',
    'System.time_now_ms': '```syntra\nSystem.time_now_ms(): int\n```\nAlias for `System.timeMs()`. Returns current Unix epoch time in milliseconds.\n\n@return 64-bit integer timestamp in milliseconds.\n@since v0.1.0',
    'System.getEnv': '```syntra\nSystem.getEnv(string key): string\n```\nReads an environment variable by name from the current process environment.\n\n```syntra\nlet home = System.getEnv("HOME");\n```\n\n@param key The case-sensitive name of the environment variable.\n@return The string value of the environment variable, or `""` if unset.\n@since v0.1.0',
    'System.env': '```syntra\nSystem.env(string key): string\n```\nAlias for `System.getEnv()`. Reads an environment variable by name.\n\n@param key The environment variable name.\n@return String value or `""` if unset.\n@since v0.1.0',
    'System.exec': '```syntra\nSystem.exec(string cmd): int\n```\nSpawns a subshell process executing the specified command and returns its termination exit code.\n\n```syntra\nlet status = System.exec("git status --short");\n```\n\n@param cmd The shell command string to execute.\n@return Process exit status code.\n@throws RuntimeException If process creation fails.\n@since v0.1.0',

    // --- Math Module ---
    'Math': '```syntra\nnamespace Math\n```\nHigh-performance IEEE-754 mathematical operations, trigonometry, and constants.\n\n```syntra\nlet hypotenuse = Math.sqrt(Math.pow(3.0, 2.0) + Math.pow(4.0, 2.0));\n```\n\n@since v0.1.0',
    'Math.sqrt': '```syntra\nMath.sqrt(float x): float\n```\nComputes the non-negative square root of $x$.\n\n```syntra\nlet r = Math.sqrt(16.0); // 4.0\n```\n\n@param x Non-negative floating-point number.\n@return The square root of x.\n@since v0.1.0',
    'Math.pow': '```syntra\nMath.pow(float base, float exp): float\n```\nComputes base raised to exponent exp ($base^{exp}$).\n\n```syntra\nlet val = Math.pow(2.0, 10.0); // 1024.0\n```\n\n@param base The base number.\n@param exp The exponent.\n@return base raised to the power exp.\n@since v0.1.0',
    'Math.abs': '```syntra\nMath.abs(float x): float\n```\nReturns the absolute value of $x$.\n\n@param x Input number.\n@return Absolute non-negative value.\n@since v0.1.0',
    'Math.min': '```syntra\nMath.min(number a, number b): number\n```\nReturns the smaller of two numbers.\n\n@param a First number.\n@param b Second number.\n@return The smaller number.\n@since v0.1.0',
    'Math.max': '```syntra\nMath.max(number a, number b): number\n```\nReturns the greater of two numbers.\n\n@param a First number.\n@param b Second number.\n@return The greater number.\n@since v0.1.0',
    'Math.floor': '```syntra\nMath.floor(float x): int\n```\nRounds down to the nearest integer.\n\n@param x Input float.\n@return Floor integer.\n@since v0.1.0',
    'Math.ceil': '```syntra\nMath.ceil(float x): int\n```\nRounds up to the nearest integer.\n\n@param x Input float.\n@return Ceil integer.\n@since v0.1.0',
    'Math.PI': 'The mathematical constant $\\pi \\approx 3.141592653589793$.',
    'Math.E': 'Euler\'s number $e \\approx 2.718281828459045$.',

    // --- File & I/O Module ---
    'File': '```syntra\nnamespace File\n```\nFast synchronous and buffered file I/O operations.\n\n```syntra\nimport File;\n\nfunc main(): void {\n    if (File.exists("Package.syn")) {\n        let text = File.readText("Package.syn");\n    }\n}\n```\n\n@since v0.1.0',
    'File.readText': '```syntra\nFile.readText(string path): string\n```\nReads the entire contents of a file into a UTF-8 string.\n\n```syntra\nlet content = File.readText("data/config.json");\n```\n\n@param path Target filesystem path.\n@return The decoded UTF-8 string contents.\n@throws RuntimeException If file cannot be read.\n@since v0.1.0',
    'File.read_to_string': '```syntra\nFile.read_to_string(string path): string\n```\nAlias for `File.readText`. Reads entire file into a UTF-8 string.\n\n@param path Target filesystem path.\n@return File contents as string.\n@since v0.1.0',
    'File.writeText': '```syntra\nFile.writeText(string path, string content): bool\n```\nWrites string text contents to a target file, creating or truncating it.\n\n```syntra\nFile.writeText("logs/out.txt", "Process completed\\n");\n```\n\n@param path Target filesystem path to write.\n@param content String data to write.\n@return `true` if write succeeded, `false` otherwise.\n@since v0.1.0',
    'File.write_string': '```syntra\nFile.write_string(string path, string content): void\n```\nAlias for `File.writeText`. Writes string content to target file path.\n\n@param path Target filesystem path.\n@param content Data to write.\n@since v0.1.0',
    'File.exists': '```syntra\nFile.exists(string path): bool\n```\nChecks whether a file exists on disk at the given path.\n\n@param path Target path to check.\n@return `true` if file exists; `false` otherwise.\n@since v0.1.0',
    'File.delete': '```syntra\nFile.delete(string path): bool\n```\nPermanently unlinks and deletes a file from the filesystem.\n\n@param path Target file path to remove.\n@return `true` if deleted successfully.\n@since v0.1.0',
    'File.size': '```syntra\nFile.size(string path): int\n```\nReturns the size of a file in bytes, or `-1` if the file does not exist.\n\n@param path Target filesystem path.\n@return 64-bit file size in bytes.\n@since v0.1.0',

    // --- Json Module ---
    'Json': '```syntra\nnamespace Json\n```\nHigh-speed native JSON serialization and parsing engine.\n\n```syntra\nlet obj = Json.parse("{\\"id\\": 101}");\nlet str = Json.stringify(obj);\n```\n\n@since v0.1.0',
    'Json.stringify': '```syntra\nJson.stringify(any obj): string\n```\nSerializes any Syntra instance, array, or map into a formatted JSON string.\n\n```syntra\nlet json = Json.stringify(myMap);\n```\n\n@param obj Object or collection to serialize.\n@return Formatted JSON string.\n@since v0.1.0',
    'Json.parse': '```syntra\nJson.parse(string jsonStr): any\n```\nParses a JSON string into Syntra Map and Array representations.\n\n```syntra\nlet map = Json.parse("{\\"status\\":\\"ok\\"}");\n```\n\n@param jsonStr Valid JSON string payload.\n@return Syntra Map or Array structure.\n@throws RuntimeException If JSON string has invalid syntax.\n@since v0.1.0',

    // --- Concurrency Primitives ---
    'Task': '```syntra\nclass Task<T>\n```\nLightweight asynchronous concurrent execution fiber.\n\n```syntra\nlet task = Task.spawn(fn() {\n    return 42;\n});\nlet result = task.join();\n```\n\n@since v0.1.0',
    'Task.spawn': '```syntra\nTask.spawn(fn(): T action): Task<T>\n```\nSpawns an asynchronous concurrent worker task executing the provided lambda.\n\n```syntra\nlet task = Task.spawn(fn() {\n    computeHeavyWork();\n});\n```\n\n@param action Lambda function closure to execute concurrently.\n@return A `Task<T>` handle.\n@since v0.1.0',
    'Task.sleep': '```syntra\nTask.sleep(int ms): void\n```\nSuspends execution of the current fiber for the specified milliseconds.\n\n@param ms Duration in milliseconds.\n@since v0.1.0',
    'Task.sleep_ms': '```syntra\nTask.sleep_ms(int ms): void\n```\nAlias for `Task.sleep`.\n\n@param ms Duration in milliseconds.\n@since v0.1.0',
    'Channel': '```syntra\nclass Channel<T>\n```\nThread-safe, FIFO message-passing synchronization channel.\n\n```syntra\nlet ch = Channel.new(10);\nch.send("Hello");\nlet msg = ch.recv();\n```\n\n@since v0.1.0',
    'Channel.new': '```syntra\nChannel.new(int capacity): Channel<T>\n```\nCreates a FIFO synchronization channel with optional buffer capacity.\n\n@param capacity Maximum buffered message capacity.\n@return A new `Channel<T>` instance.\n@since v0.1.0',
    'Channel.select': '```syntra\nChannel.select(Array<Channel<any>> channels): int\n```\nNon-blocking multiplexer over channels, returning the ready index.\n\n@param channels Array of channels to poll.\n@return Zero-based index of ready channel, or -1 if none ready.\n@since v0.1.0',
    'ThreadPool': '```syntra\nclass ThreadPool\n```\nBounded worker thread pool for parallel CPU-bound compute tasks.\n\n```syntra\nlet pool = new ThreadPool(4);\npool.submit(fn() {\n    processItem();\n});\npool.wait_idle();\n```\n\n@since v0.1.0',
    'ThreadPool.new': '```syntra\nThreadPool.new(int workers): ThreadPool\n```\nCreates a bounded worker thread pool with `workers` execution threads.\n\n@param workers Number of dedicated worker threads.\n@return New `ThreadPool` instance.\n@since v0.1.0',

    // --- HTTP & Web Subsystem ---
    'HttpRequest': '```syntra\nclass HttpRequest\n```\nRepresents an HTTP request message containing method, path, headers, and body.\n\n@since v0.1.0',
    'HttpResponse': '```syntra\nclass HttpResponse\n```\nRepresents an HTTP response message containing statusCode, statusText, headers, and body.\n\n@since v0.1.0',
    'HttpServer': '```syntra\nclass HttpServer\n```\nHigh-performance asynchronous HTTP server with keep-alive support.\n\n```syntra\nlet server = new HttpServer();\nserver.listen(8080);\n```\n\n@since v0.1.0',
    'HttpClient': '```syntra\nclass HttpClient\n```\nSynchronous HTTP client for executing GET, POST, PUT, DELETE, PATCH, and OPTIONS requests.\n\n```syntra\nlet res = HttpClient.get("example.com", 80, "/");\n```\n\n@since v0.1.0',
    'WebSocket': '```syntra\nclass WebSocket\n```\nFull native RFC 6455 WebSocket client/server connection handle with frame masking.\n\n@since v0.1.0',
    'WebSocketHub': '```syntra\nclass WebSocketHub\n```\nMulti-client WebSocket connection hub supporting broadcast and client registry.\n\n@since v0.1.0',

    // --- Primitive Types ---
    'int': '`int` — 64-bit signed two\'s-complement integer (-9223372036854775808 to 9223372036854775807).',
    'float': '`float` — 64-bit IEEE-754 double-precision floating-point number.',
    'bool': '`bool` — Boolean truth value (`true` or `false`).',
    'string': '`string` — Immutable, UTF-8 encoded string sequence.',
    'void': '`void` — Unit return type denoting absence of a value.',
};
