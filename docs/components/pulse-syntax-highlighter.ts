import Prism from 'prismjs';

// Pulse language syntax highlighting for Prism
if (typeof Prism !== 'undefined') {
  Prism.languages.pulse = {
    'comment': {
      pattern: /\/\/.*|\/\*[\s\S]*?\*\//,
      greedy: true
    },
    'string': {
      pattern: /"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/,
      greedy: true
    },
    'keyword': /\b(?:let|if|else|while|for|fun|class|return|print|spawn|send|receive|panic|import|export)\b/,
    'variable': /\b[a-zA-Z_][a-zA-Z0-9_]*\b(?!\()/,
    'function': /\b\w+(?=\()/,
    'number': /\b\d+(\.\d+)?\b/,
    'operator': /[+\-*/%=!<>]=?|&&|\|\|/,
    'punctuation': /[;(),.:[\]{}]/,
    'boolean': /\b(?:true|false)\b/,
  };
}

export function highlightPulseCode(code: string): string {
  return Prism.highlight(code, Prism.languages.pulse, 'pulse');
}
