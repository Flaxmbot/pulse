"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
function resolveServerCommand(context) {
    const configured = vscode_1.workspace.getConfiguration('pulse').get('lsp.path');
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
function activate(context) {
    const serverPath = resolveServerCommand(context);
    const serverOptions = {
        run: { command: serverPath, transport: node_1.TransportKind.stdio },
        debug: { command: serverPath, transport: node_1.TransportKind.stdio }
    };
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'pulse' }],
        synchronize: {
            fileEvents: vscode_1.workspace.createFileSystemWatcher('**/*.pulse')
        }
    };
    client = new node_1.LanguageClient('pulseLsp', 'Pulse Language Server', serverOptions, clientOptions);
    client.start();
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map