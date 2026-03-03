"use client";

import { useState, useEffect } from "react";
import Editor from "react-simple-code-editor";
import { highlightPulseCode } from "./pulse-syntax-highlighter";
import "prismjs/themes/prism-tomorrow.css";

const DEFAULT_CODE = `// Pulse Playground
// Try running this code!

let total = 0;
let i = 1;
while (i <= 5) {
  total = total + i;
  i = i + 1;
}

print("sum(1..5) =", total);

// Actor example
let worker = spawn {
  while (true) {
    let msg = receive;
    print("Worker received:", msg);
  }
};

send worker, {"type": "job", "payload": "task-1"};
send worker, {"type": "job", "payload": "task-2"};
`;

export function Playground() {
  const [code, setCode] = useState(DEFAULT_CODE);
  const [output, setOutput] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [isRunning, setIsRunning] = useState(false);

  const runCode = async () => {
    setIsRunning(true);
    setOutput("");
    setError("");

    try {
      // In a real implementation, this would communicate with a Pulse language server or API
      // For this demo, we'll simulate execution with a timeout
      await new Promise(resolve => setTimeout(resolve, 1500));
      
      // Simulate successful execution
      setOutput(`Execution started...
sum(1..5) = 15
Worker received: {"type":"job","payload":"task-1"}
Worker received: {"type":"job","payload":"task-2"}
Execution completed successfully!`);
    } catch (err) {
      setError(`Error: ${(err as Error).message}`);
    } finally {
      setIsRunning(false);
    }
  };

  const resetCode = () => {
    setCode(DEFAULT_CODE);
    setOutput("");
    setError("");
  };

  return (
    <div className="playground">
      <div className="playground-header">
        <h2>Pulse Playground</h2>
        <div className="playground-controls">
          <button 
            className="btn-run" 
            onClick={runCode} 
            disabled={isRunning}
          >
            {isRunning ? "Running..." : "Run"}
          </button>
          <button 
            className="btn-reset" 
            onClick={resetCode}
            disabled={isRunning}
          >
            Reset
          </button>
        </div>
      </div>
      
      <div className="playground-body">
        <div className="editor-section">
          <div className="section-label">Code</div>
          <Editor
            value={code}
            onValueChange={setCode}
            highlight={highlightPulseCode}
            padding={16}
            className="code-editor"
            style={{
              fontFamily: '"Fira Code", "Fira Mono", monospace',
              fontSize: 14,
              minHeight: '300px',
            }}
          />
        </div>
        
        <div className="output-section">
          <div className="section-label">Output</div>
          {error && (
            <div className="error-output">
              {error}
            </div>
          )}
          {output && (
            <div className="normal-output">
              {output}
            </div>
          )}
          {!output && !error && !isRunning && (
            <div className="empty-output">
              Click "Run" to execute your code
            </div>
          )}
          {isRunning && (
            <div className="running-output">
              Executing code...
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
