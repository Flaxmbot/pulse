use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

mod diagnostics;

pub use diagnostics::*;

/// Document state tracked by the LSP
#[derive(Default)]
pub struct DocumentState {
    pub content: String,
    pub version: i32,
}

/// Pulse Language Server backend
pub struct PulseBackend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

impl PulseBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for PulseBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "pulse-lsp".into(),
                version: Some("0.1.0".into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Pulse LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text.clone();
        let version = params.text_document.version;
        
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), DocumentState { content: content.clone(), version });
        }
        
        // Run diagnostics
        let diagnostics = diagnose_source(&content);
        self.client.publish_diagnostics(uri, diagnostics, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        
        if let Some(change) = params.content_changes.into_iter().last() {
            let content = change.text;
            {
                let mut docs = self.documents.write().await;
                docs.insert(uri.clone(), DocumentState { content: content.clone(), version });
            }
            
            // Run diagnostics
            let diagnostics = diagnose_source(&content);
            self.client.publish_diagnostics(uri, diagnostics, Some(version)).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        
        let content = {
            let docs = self.documents.read().await;
            docs.get(&uri).map(|d| d.content.clone())
        };
        
        if let Some(content) = content {
            let diagnostics = diagnose_source(&content);
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        
        let _content = {
            let docs = self.documents.read().await;
            docs.get(&uri).map(|d| d.content.clone())
        };
        
        // Basic keyword completions
        let completions = vec![
            CompletionItem::new_simple("fn".into(), "Function definition".into()),
            CompletionItem::new_simple("def".into(), "Function definition (alias)".into()),
            CompletionItem::new_simple("let".into(), "Variable declaration".into()),
            CompletionItem::new_simple("if".into(), "If statement".into()),
            CompletionItem::new_simple("else".into(), "Else clause".into()),
            CompletionItem::new_simple("while".into(), "While loop".into()),
            CompletionItem::new_simple("for".into(), "For loop".into()),
            CompletionItem::new_simple("in".into(), "In keyword for for-in loops".into()),
            CompletionItem::new_simple("return".into(), "Return statement".into()),
            CompletionItem::new_simple("actor".into(), "Actor definition".into()),
            CompletionItem::new_simple("spawn".into(), "Spawn actor".into()),
            CompletionItem::new_simple("send".into(), "Send message".into()),
            CompletionItem::new_simple("receive".into(), "Receive block".into()),
            CompletionItem::new_simple("match".into(), "Pattern matching".into()),
            CompletionItem::new_simple("try".into(), "Try block".into()),
            CompletionItem::new_simple("catch".into(), "Catch block".into()),
            CompletionItem::new_simple("print".into(), "Print statement".into()),
            CompletionItem::new_simple("true".into(), "Boolean true".into()),
            CompletionItem::new_simple("false".into(), "Boolean false".into()),
            CompletionItem::new_simple("nil".into(), "Nil value".into()),
        ];

        Ok(Some(CompletionResponse::Array(completions)))
    }
}
