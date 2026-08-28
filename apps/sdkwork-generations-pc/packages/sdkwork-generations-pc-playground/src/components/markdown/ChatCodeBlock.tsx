import { useState } from 'react';
import type { ReactNode } from 'react';
import { Check, Copy } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useOptionalPlaygroundHost } from '../../PlaygroundHostContext.tsx';

const CODE_COPY_RESET_MS = 1400;

export function ChatCodeBlock({
  code,
  language,
  tone,
}: {
  code: string;
  language?: string;
  tone: 'assistant' | 'user' | 'danger';
}) {
  const host = useOptionalPlaygroundHost();
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const displayCode = normalizeCodeBlockLineSeparators(code);
  const languageLabel = normalizeLanguageLabel(language);

  async function handleCopy(): Promise<void> {
    if (!displayCode) {
      return;
    }
    const result = host
      ? await host.copyTextToClipboard(displayCode)
      : await copyTextWithNavigator(displayCode);
    if (!result.ok) {
      setCopied(false);
      return;
    }
    setCopied(true);
    globalThis.setTimeout(() => {
      setCopied(false);
    }, CODE_COPY_RESET_MS);
  }

  const copyLabel = copied ? t('common.actions.copied') : t('common.actions.copyCode');

  return (
    <figure
      className="sdkwork-playground-chat-code-block my-3 min-w-0 overflow-hidden rounded-xl border border-current/10 bg-current/[0.04] text-left shadow-sm"
      data-tone={tone}
    >
      <figcaption className="sdkwork-playground-chat-code-block__header flex items-center justify-between border-b border-current/10 bg-current/[0.06] px-4 py-2 text-xs">
        <span className="sdkwork-playground-chat-code-block__language font-mono font-medium text-cyan-500 dark:text-cyan-400">
          {languageLabel}
        </span>
        <button
          type="button"
          title={copyLabel}
          aria-label={copyLabel}
          onClick={() => {
            void handleCopy();
          }}
          className="sdkwork-playground-chat-code-block__copy inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-current/70 transition-colors hover:bg-current/10 hover:text-current"
        >
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          <span>{copyLabel}</span>
        </button>
      </figcaption>
      <pre
        tabIndex={0}
        className="max-w-full overflow-x-auto bg-[#1e1e1e] px-0 py-3 text-[13.5px] leading-[1.625rem] text-gray-100 [tab-size:2] dark:bg-[#0f0f0f]"
      >
        <code className="block min-w-max whitespace-pre font-mono">
          {renderHighlightedCodeLines(displayCode || '\u00a0', language)}
        </code>
      </pre>
    </figure>
  );
}

function normalizeLanguageLabel(language: string | undefined): string {
  const normalized = language?.trim().replace(/^language-/, '') || '';
  return normalized || 'text';
}

function normalizeCodeBlockLineSeparators(value: string): string {
  if (!/\\[rnt]/.test(value)) {
    return value;
  }

  let result = '';
  let quote: '"' | "'" | '`' | null = null;
  let escaped = false;

  for (let index = 0; index < value.length; index += 1) {
    const current = value[index];
    const next = value[index + 1];

    if (quote) {
      result += current;
      if (escaped) {
        escaped = false;
      } else if (current === '\\') {
        escaped = true;
      } else if (current === quote) {
        quote = null;
      }
      continue;
    }

    if (current === '"' || current === "'" || current === '`') {
      quote = current;
      result += current;
      continue;
    }

    if (current === '\\' && next === 'r') {
      if (value[index + 2] === '\\' && value[index + 3] === 'n') {
        result += '\n';
        index += 3;
      } else {
        result += '\n';
        index += 1;
      }
      continue;
    }

    if (current === '\\' && next === 'n') {
      result += '\n';
      index += 1;
      continue;
    }

    if (current === '\\' && next === 't') {
      result += '\t';
      index += 1;
      continue;
    }

    result += current;
  }

  return result;
}

function renderHighlightedCodeLines(code: string, language: string | undefined): ReactNode[] {
  return code.split('\n').map((line, lineIndex) => (
    <span
      key={`code-line-${lineIndex}`}
      data-chat-code-line={lineIndex + 1}
      className="block min-h-[1.625rem] whitespace-pre px-4"
    >
      {highlightCodeLine(line, language, lineIndex)}
    </span>
  ));
}

function highlightCodeLine(line: string, language: string | undefined, lineIndex: number): ReactNode[] {
  const result: ReactNode[] = [];
  const tokenPattern = /("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|\/\/.*$|#.*$|\b\d+(?:\.\d+)?\b|\b[A-Za-z_$][\w$]*\b|[{}()[\].,;:<>/=+\-*%!?&|]+)/g;
  let cursor = 0;
  let tokenIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = tokenPattern.exec(line)) !== null) {
    if (match.index > cursor) {
      result.push(line.slice(cursor, match.index));
    }
    const token = match[0];
    const className = codeTokenClassName(token, language);
    result.push(className ? (
      <span key={`${lineIndex}-${tokenIndex}`} className={className}>{token}</span>
    ) : token);
    cursor = match.index + token.length;
    tokenIndex += 1;
  }

  if (cursor < line.length) {
    result.push(line.slice(cursor));
  }
  return result.length > 0 ? result : [''];
}

function codeTokenClassName(token: string, language: string | undefined): string | undefined {
  if (/^(\/\/|#)/.test(token)) {
    return 'sdkwork-playground-chat-code-token--comment';
  }
  if (/^(['"`])/.test(token)) {
    return 'sdkwork-playground-chat-code-token--string';
  }
  if (/^\d/.test(token)) {
    return 'sdkwork-playground-chat-code-token--number';
  }
  if (isCodeKeyword(token, language)) {
    return 'sdkwork-playground-chat-code-token--keyword';
  }
  if (isCodeBuiltin(token)) {
    return 'sdkwork-playground-chat-code-token--builtin';
  }
  if (/^[{}()[\].,;:<>/=+\-*%!?&|]+$/.test(token)) {
    return 'sdkwork-playground-chat-code-token--punct';
  }
  return undefined;
}

function isCodeKeyword(token: string, language: string | undefined): boolean {
  const normalizedLanguage = language?.toLowerCase() || '';
  const sharedKeywords = new Set([
    'as',
    'async',
    'await',
    'break',
    'case',
    'catch',
    'class',
    'const',
    'continue',
    'default',
    'do',
    'else',
    'export',
    'extends',
    'false',
    'finally',
    'for',
    'from',
    'function',
    'if',
    'import',
    'in',
    'interface',
    'let',
    'new',
    'null',
    'return',
    'switch',
    'throw',
    'true',
    'try',
    'type',
    'undefined',
    'while',
  ]);
  if (sharedKeywords.has(token)) {
    return true;
  }
  if (['py', 'python'].includes(normalizedLanguage)) {
    return ['def', 'elif', 'except', 'global', 'lambda', 'nonlocal', 'pass', 'with', 'yield'].includes(token);
  }
  if (['rs', 'rust'].includes(normalizedLanguage)) {
    return ['fn', 'impl', 'let', 'match', 'mod', 'mut', 'pub', 'self', 'struct', 'trait', 'use', 'where'].includes(token);
  }
  if (['sql'].includes(normalizedLanguage)) {
    return [
      'SELECT',
      'FROM',
      'WHERE',
      'INSERT',
      'UPDATE',
      'DELETE',
      'JOIN',
      'LEFT',
      'RIGHT',
      'GROUP',
      'ORDER',
      'BY',
      'LIMIT',
      'VALUES',
    ].includes(token.toUpperCase());
  }
  return false;
}

function isCodeBuiltin(token: string): boolean {
  return [
    'Array',
    'Boolean',
    'Date',
    'Error',
    'Map',
    'Number',
    'Object',
    'Promise',
    'Record',
    'Set',
    'String',
    'console',
    'fetch',
  ].includes(token);
}

async function copyTextWithNavigator(text: string): Promise<{ ok: boolean }> {
  if (!text || !globalThis.navigator?.clipboard?.writeText) {
    return { ok: false };
  }
  try {
    await globalThis.navigator.clipboard.writeText(text);
    return { ok: true };
  } catch {
    return { ok: false };
  }
}
