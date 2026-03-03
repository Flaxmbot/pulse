import * as path from 'path';
import * as fs from 'fs';
import { workspace, ExtensionContext } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

function resolveServerCommand(context: ExtensionContext): string {
    const configured = workspace.getConfiguration('pulse').get<string>('lsp.path');
    if (configured && configured.trim().length > 0) {
        return configured;
    }

    const envPath = process.env.PULSE_LSP_PATH;
    if (envPath && envPath.trim().length > 0) {
        return envPath;
    }

    const extRoot = context.extensionPath;
    const candidates = process.platform === 'win32'
        ? [
            path.join(extRoot, '..', 'pulse_lang', 'target', 'release', 'pulse-lsp.exe'),
            path.join(extRoot, '..', 'pulse_lang', 'target', 'debug', 'pulse-lsp.exe'),
        ]
        : [
            path.join(extRoot, '..', 'pulse_lang', 'target', 'release', 'pulse-lsp'),
            path.join(extRoot, '..', 'pulse_lang', 'target', 'debug', 'pulse-lsp'),
        ];

    for (const candidate of candidates) {
        if (fs.existsSync(candidate)) {
            return candidate;
        }
    }

    // Fallback to PATH lookup
    return process.platform === 'win32' ? 'pulse-lsp.exe' : 'pulse-lsp';
}

export function activate(context: ExtensionContext) {
    const serverPath = resolveServerCommand(context);

    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio },
        debug: { command: serverPath, transport: TransportKind.stdio }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'pulse' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.pulse')
        }
    };

    client = new LanguageClient(
        'pulseLsp',
        'Pulse Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
