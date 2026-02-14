import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    // Path to the compiled LSP binary
    const serverPath = context.asAbsolutePath(
        path.join('..', 'pulse_lang', 'target', 'debug', 'pulse-lsp.exe')
    );

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
