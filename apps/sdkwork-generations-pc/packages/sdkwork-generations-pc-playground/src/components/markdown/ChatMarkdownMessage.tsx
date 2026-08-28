import { Children, cloneElement, createContext, isValidElement, useContext, useMemo } from 'react';
import type {
  AnchorHTMLAttributes,
  ComponentPropsWithoutRef,
  HTMLAttributes,
  ReactElement,
  ImgHTMLAttributes,
  ReactNode,
} from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';
import { ChatCodeBlock } from './ChatCodeBlock.tsx';

type ChatMarkdownTone = 'assistant' | 'user' | 'danger';
type MarkdownComponentProps<T extends HTMLElement> = HTMLAttributes<T> & {
  node?: unknown;
};
type MarkdownInputProps = ComponentPropsWithoutRef<'input'> & {
  node?: unknown;
};
type MarkdownCodeProps = ComponentPropsWithoutRef<'code'> & {
  node?: unknown;
  tone: ChatMarkdownTone;
};
type MarkdownLinkProps = AnchorHTMLAttributes<HTMLAnchorElement> & {
  node?: unknown;
};
type MarkdownImageProps = ImgHTMLAttributes<HTMLImageElement> & {
  display?: 'block' | 'inline';
  node?: unknown;
};

const INLINE_MATH_PREFIX = '__CHAT_MATH_INLINE_';
const MATH_DISPLAY_LANGUAGE = 'math-display';
const BlockCodeContext = createContext(false);

export function ChatMarkdownMessage({
  content,
  tone,
  streaming = false,
}: {
  content: string;
  tone: ChatMarkdownTone;
  streaming?: boolean;
}) {
  const markdown = useMemo(
    () => {
      const normalizedContent = normalizeChatMarkdownContent(content);
      return sanitizeMarkdownInput(streaming ? normalizeStreamingMarkdown(normalizedContent) : normalizedContent);
    },
    [content, streaming],
  );
  const components = useMemo(() => ({
    a: (props: MarkdownLinkProps) => <ChatMarkdownLink {...props} />,
    blockquote: (props: MarkdownComponentProps<HTMLQuoteElement>) => (
      <ChatMarkdownBlockquote tone={tone} {...props} />
    ),
    br: (props: MarkdownComponentProps<HTMLBRElement>) => <ChatMarkdownBreak {...props} />,
    code: (props: Omit<MarkdownCodeProps, 'tone'>) => <ChatMarkdownCode tone={tone} {...props} />,
    del: (props: MarkdownComponentProps<HTMLModElement>) => <ChatMarkdownDeleted {...props} />,
    em: (props: MarkdownComponentProps<HTMLElement>) => <ChatMarkdownEmphasis {...props} />,
    h1: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={1} {...props} />,
    h2: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={2} {...props} />,
    h3: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={3} {...props} />,
    h4: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={4} {...props} />,
    h5: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={5} {...props} />,
    h6: (props: MarkdownComponentProps<HTMLHeadingElement>) => <ChatMarkdownHeading level={6} {...props} />,
    hr: (props: MarkdownComponentProps<HTMLHRElement>) => <ChatMarkdownDivider tone={tone} {...props} />,
    img: ChatMarkdownImage,
    input: (props: MarkdownInputProps) => <ChatMarkdownCheckbox {...props} />,
    li: (props: MarkdownComponentProps<HTMLLIElement>) => <ChatMarkdownListItem {...props} />,
    ol: (props: MarkdownComponentProps<HTMLOListElement>) => <ChatMarkdownList ordered {...props} />,
    p: (props: MarkdownComponentProps<HTMLParagraphElement>) => <ChatMarkdownParagraph {...props} />,
    pre: (props: MarkdownComponentProps<HTMLPreElement>) => <ChatMarkdownPre {...props} />,
    section: (props: MarkdownComponentProps<HTMLElement>) => <ChatMarkdownSection {...props} />,
    strong: (props: MarkdownComponentProps<HTMLElement>) => <ChatMarkdownStrong {...props} />,
    sup: (props: MarkdownComponentProps<HTMLElement>) => <ChatMarkdownSuperscript {...props} />,
    table: (props: MarkdownComponentProps<HTMLTableElement>) => <ChatMarkdownTable {...props} />,
    tbody: (props: MarkdownComponentProps<HTMLTableSectionElement>) => {
      const { node: _node, ...rest } = props;
      return <tbody {...rest} />;
    },
    td: (props: MarkdownComponentProps<HTMLTableCellElement>) => <ChatMarkdownTableCell {...props} />,
    th: (props: MarkdownComponentProps<HTMLTableCellElement>) => <ChatMarkdownTableHeader {...props} />,
    thead: (props: MarkdownComponentProps<HTMLTableSectionElement>) => {
      const { node: _node, ...rest } = props;
      return <thead {...rest} />;
    },
    tr: (props: MarkdownComponentProps<HTMLTableRowElement>) => <ChatMarkdownTableRow {...props} />,
    ul: (props: MarkdownComponentProps<HTMLUListElement>) => <ChatMarkdownList {...props} />,
  }), [tone]);

  return (
    <div
      className="sdkwork-playground-chat-markdown chat-markdown"
      data-tone={tone}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[[rehypeSanitize, chatMarkdownSanitizeSchema]]}
        components={components}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
}

export const chatMarkdownSanitizeSchema = {
  ...defaultSchema,
  tagNames: [
    ...(defaultSchema.tagNames || []),
    'input',
    'section',
  ],
  attributes: {
    ...defaultSchema.attributes,
    a: [
      ...(defaultSchema.attributes?.a || []),
      ['target'],
      ['rel'],
    ],
    code: [
      ...(defaultSchema.attributes?.code || []),
      ['className'],
    ],
    input: [
      ...(defaultSchema.attributes?.input || []),
      ['checked'],
      ['disabled'],
      ['type'],
    ],
    section: [
      ...(defaultSchema.attributes?.section || []),
      ['className'],
      ['dataFootnotes'],
    ],
  },
  protocols: {
    ...defaultSchema.protocols,
    href: ['http', 'https', 'mailto'],
  },
};

function ChatMarkdownParagraph(props: MarkdownComponentProps<HTMLParagraphElement>) {
  const { children, className, node: _node, ...rest } = props;
  const visibleChildren = readVisibleMarkdownChildren(children);
  if (visibleChildren.length === 0) {
    return null;
  }
  if (isImageOnlyMarkdownParagraph(visibleChildren)) {
    if (!visibleChildren.some(hasSafeMarkdownImageSource)) {
      return null;
    }
    return (
      <div
        {...rest}
        className={mergeClassNames('my-4 min-w-0 first:mt-0 last:mb-0', className)}
      >
        {renderMarkdownImageChildren(children, 'block')}
      </div>
    );
  }
  return (
    <p
      {...rest}
      className={mergeClassNames('my-3 min-w-0 break-words [overflow-wrap:anywhere] first:mt-0 last:mb-0', className)}
    >
      {children}
    </p>
  );
}

function ChatMarkdownBreak(props: MarkdownComponentProps<HTMLBRElement>) {
  const { node: _node, ...rest } = props;
  return <br {...rest} />;
}

function ChatMarkdownHeading({
  level,
  children,
  className,
  node: _node,
  ...props
}: MarkdownComponentProps<HTMLHeadingElement> & {
  level: 1 | 2 | 3 | 4 | 5 | 6;
}) {
  const Tag = `h${level}` as const;
  const sizeClass = level === 1
    ? 'text-[20px]'
    : level === 2
      ? 'text-[18px]'
      : level === 3
        ? 'text-[16px]'
        : level === 4
          ? 'text-[15px]'
          : 'text-[14px]';
  return (
    <Tag
      {...props}
      className={mergeClassNames('mb-2 mt-5 font-semibold leading-7 tracking-normal first:mt-0', sizeClass, className)}
    >
      {children}
    </Tag>
  );
}

function ChatMarkdownList({
  ordered = false,
  children,
  className,
  node: _node,
  ...props
}: MarkdownComponentProps<HTMLUListElement | HTMLOListElement> & {
  ordered?: boolean;
}) {
  const Tag = ordered ? 'ol' : 'ul';
  return (
    <Tag
      {...props}
      className={mergeClassNames(
        'my-3 space-y-1.5 first:mt-0 last:mb-0',
        ordered ? 'list-decimal pl-6' : 'list-disc pl-5',
        className,
      )}
    >
      {children}
    </Tag>
  );
}

function ChatMarkdownListItem(props: MarkdownComponentProps<HTMLLIElement>) {
  const { className, node: _node, ...rest } = props;
  const isTaskListItem = className?.includes('task-list-item');
  return (
    <li
      {...rest}
      className={mergeClassNames(
        'min-w-0 marker:text-current/55',
        isTaskListItem ? 'list-none pl-0' : 'pl-1',
        'break-words [overflow-wrap:anywhere]',
        '[&>p]:my-1.5 [&>p:first-child]:mt-0 [&>p:last-child]:mb-0 [&>ul]:my-2 [&>ol]:my-2 [&>figure]:my-2',
        className,
      )}
    />
  );
}

function ChatMarkdownCheckbox(props: MarkdownInputProps) {
  const { className, node: _node, ...rest } = props;
  return (
    <input
      {...rest}
      readOnly
      className={mergeClassNames('mr-2 h-3.5 w-3.5 translate-y-0.5 rounded border-current/30', className)}
    />
  );
}

function ChatMarkdownBlockquote({
  tone,
  className,
  node: _node,
  ...props
}: MarkdownComponentProps<HTMLQuoteElement> & {
  tone: ChatMarkdownTone;
}) {
  return (
    <blockquote
      {...props}
      data-tone={tone}
      className={mergeClassNames(
        'sdkwork-playground-chat-markdown__blockquote my-4 first:mt-0 last:mb-0',
        className,
      )}
    />
  );
}

function ChatMarkdownDivider({
  tone,
  className,
  node: _node,
  ...props
}: MarkdownComponentProps<HTMLHRElement> & {
  tone: ChatMarkdownTone;
}) {
  return (
    <hr
      {...props}
      data-tone={tone}
      className={mergeClassNames('sdkwork-playground-chat-markdown__hr', className)}
    />
  );
}

function ChatMarkdownPre({ children }: MarkdownComponentProps<HTMLPreElement>) {
  return (
    <BlockCodeContext.Provider value={true}>
      {children}
    </BlockCodeContext.Provider>
  );
}

function ChatMarkdownCode({
  children,
  className,
  node: _node,
  tone,
  ...props
}: MarkdownCodeProps) {
  const inBlock = useContext(BlockCodeContext);
  const rawCode = flattenText(children);
  const code = rawCode.replace(/\n$/, '');
  const language = /language-([a-zA-Z0-9_-]+)/.exec(className || '')?.[1];
  const displayMath = language === MATH_DISPLAY_LANGUAGE ? decodeMathPayload(code.trim()) : null;
  const inlineMath = decodeInlineMathPlaceholder(code.trim());
  const isBlock = inBlock || Boolean(className?.includes('language-') || rawCode.includes('\n') || rawCode.endsWith('\n'));

  if (displayMath !== null) {
    return <ChatMarkdownMath tex={displayMath} tone={tone} display />;
  }

  if (inlineMath !== null) {
    return <ChatMarkdownMath tex={inlineMath} tone={tone} />;
  }

  if (isBlock) {
    return <ChatCodeBlock code={code} language={language} tone={tone} />;
  }

  return (
    <code
      {...props}
      data-tone={tone}
      className={mergeClassNames(
        'sdkwork-playground-chat-markdown__inline-code rounded border border-current/10 bg-current/[0.06] px-1.5 py-0.5 font-mono text-[0.88em]',
        className,
      )}
    >
      {children}
    </code>
  );
}

export function ChatMarkdownTable(props: MarkdownComponentProps<HTMLTableElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <div
      tabIndex={0}
      className="my-4 max-w-full overflow-x-auto rounded-xl border border-current/10 first:mt-0 last:mb-0"
    >
      <table
        {...rest}
        className={mergeClassNames('min-w-full border-collapse text-left text-[13px] leading-5', className)}
      />
    </div>
  );
}

function ChatMarkdownTableHeader(props: MarkdownComponentProps<HTMLTableCellElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <th
      {...rest}
      className={mergeClassNames(
        'min-w-[9rem] max-w-[28rem] whitespace-normal break-words border-r border-current/10 bg-current/[0.07] px-3 py-2 font-semibold [overflow-wrap:anywhere] last:border-r-0',
        className,
      )}
    />
  );
}

function ChatMarkdownTableCell(props: MarkdownComponentProps<HTMLTableCellElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <td
      {...rest}
      className={mergeClassNames(
        'min-w-[9rem] max-w-[28rem] whitespace-normal break-words border-r border-current/10 px-3 py-2 align-top [overflow-wrap:anywhere] last:border-r-0',
        className,
      )}
    />
  );
}

function ChatMarkdownTableRow(props: MarkdownComponentProps<HTMLTableRowElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <tr
      {...rest}
      className={mergeClassNames('border-b border-current/10 last:border-b-0', className)}
    />
  );
}

function ChatMarkdownLink({
  href,
  children,
  className,
  node: _node,
  ...props
}: MarkdownLinkProps) {
  if (!href || !isSafeMarkdownHref(href)) {
    return <span>{children}</span>;
  }
  const isInternalLink = isSafeMarkdownFragmentHref(href);
  const linkClassName = mergeClassNames(
    'font-medium underline decoration-current/35 underline-offset-4 transition-colors hover:decoration-current',
    'break-words [overflow-wrap:anywhere]',
    className,
  );
  if (isInternalLink) {
    return (
      <a
        {...props}
        href={href}
        className={linkClassName}
      >
        {children}
      </a>
    );
  }
  return (
    <a
      {...props}
      href={href}
      target="_blank"
      rel="noreferrer noopener"
      className={linkClassName}
    >
      {children}
    </a>
  );
}

function ChatMarkdownStrong(props: MarkdownComponentProps<HTMLElement>) {
  const { node: _node, ...rest } = props;
  return <strong {...rest} />;
}

function ChatMarkdownEmphasis(props: MarkdownComponentProps<HTMLElement>) {
  const { className, node: _node, ...rest } = props;
  return <em {...rest} className={mergeClassNames('italic text-current/95', className)} />;
}

function ChatMarkdownDeleted(props: MarkdownComponentProps<HTMLModElement>) {
  const { className, node: _node, ...rest } = props;
  return <del {...rest} className={mergeClassNames('text-current/65 line-through decoration-current/45', className)} />;
}

function ChatMarkdownSuperscript(props: MarkdownComponentProps<HTMLElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <sup
      {...rest}
      className={mergeClassNames('align-super text-[0.72em] leading-none text-current/80', className)}
    />
  );
}

function ChatMarkdownSection(props: MarkdownComponentProps<HTMLElement>) {
  const { className, node: _node, ...rest } = props;
  return (
    <section
      {...rest}
      className={mergeClassNames(
        'mt-6 border-t border-current/10 pt-4 text-[13px] leading-6 text-current/75',
        className,
      )}
    />
  );
}

function ChatMarkdownImage({
  src,
  alt,
  className,
  display = 'inline',
  node: _node,
  ...props
}: MarkdownImageProps) {
  if (!src || !isSafeMarkdownImageSrc(src)) {
    return null;
  }
  return (
    <img
      {...props}
      src={src}
      alt={alt || ''}
      loading="lazy"
      className={mergeClassNames(
        display === 'block'
          ? 'block h-auto max-h-[480px] max-w-full rounded-xl border border-current/10 object-contain'
          : 'inline-block h-auto max-h-[18rem] max-w-full align-middle rounded-lg border border-current/10 object-contain',
        className,
      )}
    />
  );
}

function ChatMarkdownMath({
  tex,
  tone,
  display = false,
}: {
  tex: string;
  tone: ChatMarkdownTone;
  display?: boolean;
}) {
  const children = renderMathExpression(tex);
  if (display) {
    return (
      <div
        tabIndex={0}
        data-tone={tone}
        className="sdkwork-playground-chat-markdown__math-display katex-display first:mt-0 last:mb-0"
      >
        <span className="katex inline-flex min-w-max items-center justify-center font-serif text-[1.08em] leading-7 text-current">
          {children}
        </span>
      </div>
    );
  }
  return (
    <span className="katex inline-flex items-baseline gap-0.5 font-serif text-[1.03em] text-current">
      {children}
    </span>
  );
}

export function normalizeStreamingMarkdown(content: string): string {
  const { activeFence } = readUnclosedMarkdownFence(content);
  if (!activeFence) {
    return content;
  }
  return `${content}\n${activeFence}`;
}

export function normalizeChatMarkdownContent(content: string): string {
  return normalizeProfessionalMarkdownSyntax(
    normalizeCompactMarkdownSyntax(
      readMarkdownTextFromUnknown(content, 0) ?? decodeTransportEscapedMarkdown(content),
    ),
  );
}

export function isSafeMarkdownHref(href: string): boolean {
  const normalized = href.trim().toLowerCase();
  return isSafeMarkdownFragmentHref(href)
    || normalized.startsWith('http://')
    || normalized.startsWith('https://')
    || normalized.startsWith('mailto:');
}

function isSafeMarkdownFragmentHref(href: string): boolean {
  const normalized = href.trim();
  return /^#[A-Za-z0-9][A-Za-z0-9._~:-]*$/.test(normalized);
}

function isSafeMarkdownImageSrc(src: string): boolean {
  const normalized = src.trim().toLowerCase();
  return normalized.startsWith('http://') || normalized.startsWith('https://');
}

function readVisibleMarkdownChildren(children: ReactNode): ReactNode[] {
  return Children.toArray(children).filter((child) => !(typeof child === 'string' && child.trim().length === 0));
}

function renderMarkdownImageChildren(children: ReactNode, display: MarkdownImageProps['display']): ReactNode {
  return Children.map(children, (child) => (
    isMarkdownImageElement(child) ? cloneElement(child, { display }) : child
  ));
}

function isImageOnlyMarkdownParagraph(children: ReactNode[]): boolean {
  return children.length > 0 && children.every(isMarkdownImageElement);
}

function hasSafeMarkdownImageSource(child: ReactNode): boolean {
  if (!isMarkdownImageElement(child)) {
    return false;
  }
  const src = child.props.src;
  return typeof src === 'string' && isSafeMarkdownImageSrc(src);
}

function isMarkdownImageElement(child: ReactNode): child is ReactElement<MarkdownImageProps> {
  return isValidElement<MarkdownImageProps>(child)
    && (child.type === ChatMarkdownImage || child.type === 'img');
}

function flattenText(children: ReactNode): string {
  if (children === null || children === undefined || typeof children === 'boolean') {
    return '';
  }
  if (typeof children === 'string' || typeof children === 'number') {
    return String(children);
  }
  if (Array.isArray(children)) {
    return children.map(flattenText).join('');
  }
  return '';
}

function sanitizeMarkdownInput(content: string): string {
  return mapMarkdownLinesOutsideCodeFences(content, escapeRawHtmlTags);
}

function decodeMathPayload(payload: string): string | null {
  try {
    return decodeURIComponent(payload);
  } catch {
    return null;
  }
}

function decodeInlineMathPlaceholder(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed.startsWith(INLINE_MATH_PREFIX) || !trimmed.endsWith('__')) {
    return null;
  }
  return decodeMathPayload(trimmed.slice(INLINE_MATH_PREFIX.length, -2));
}

function renderMathExpression(tex: string): ReactNode {
  return renderMathTextWithScripts(normalizeReadableMathText(tex));
}

const MATH_COMMAND_REPLACEMENTS: Record<string, string> = {
  alpha: '\u03b1',
  beta: '\u03b2',
  gamma: '\u03b3',
  delta: '\u03b4',
  epsilon: '\u03b5',
  theta: '\u03b8',
  lambda: '\u03bb',
  mu: '\u03bc',
  pi: '\u03c0',
  rho: '\u03c1',
  sigma: '\u03c3',
  tau: '\u03c4',
  phi: '\u03c6',
  omega: '\u03c9',
  Gamma: '\u0393',
  Delta: '\u0394',
  Theta: '\u0398',
  Lambda: '\u039b',
  Pi: '\u03a0',
  Sigma: '\u03a3',
  Phi: '\u03a6',
  Omega: '\u03a9',
  cdot: '\u00b7',
  times: '\u00d7',
  div: '\u00f7',
  pm: '\u00b1',
  mp: '\u2213',
  le: '\u2264',
  leq: '\u2264',
  ge: '\u2265',
  geq: '\u2265',
  neq: '\u2260',
  approx: '\u2248',
  sim: '\u223c',
  infty: '\u221e',
  int: '\u222b',
  sum: '\u2211',
  prod: '\u220f',
  partial: '\u2202',
  nabla: '\u2207',
  to: '\u2192',
  rightarrow: '\u2192',
  leftarrow: '\u2190',
  leftrightarrow: '\u2194',
  forall: '\u2200',
  exists: '\u2203',
  in: '\u2208',
  notin: '\u2209',
  subset: '\u2282',
  subseteq: '\u2286',
  cup: '\u222a',
  cap: '\u2229',
};

function normalizeReadableMathText(tex: string): string {
  let value = tex.trim();
  value = replaceTexTwoGroupCommand(value, 'frac', (numerator, denominator) => `${numerator} / ${denominator}`);
  value = replaceTexOneGroupCommand(value, 'sqrt', (radicand) => `\u221a(${radicand})`);
  value = value
    .replace(/\\(?:left|right)\s*/g, '')
    .replace(/\\([A-Za-z]+)/g, (match, command: string) => MATH_COMMAND_REPLACEMENTS[command] || command || match)
    .replace(/\\([{}_^$%&#])/g, '$1')
    .replace(/\s+/g, ' ')
    .trim();
  return value || tex;
}

function replaceTexTwoGroupCommand(
  value: string,
  command: string,
  format: (first: string, second: string) => string,
): string {
  const pattern = new RegExp(`\\\\${command}\\s*\\{([^{}]*)\\}\\s*\\{([^{}]*)\\}`, 'g');
  let previous = value;
  let next = previous.replace(pattern, (_match, first: string, second: string) => format(first, second));
  while (next !== previous) {
    previous = next;
    next = previous.replace(pattern, (_match, first: string, second: string) => format(first, second));
  }
  return next;
}

function replaceTexOneGroupCommand(
  value: string,
  command: string,
  format: (content: string) => string,
): string {
  const pattern = new RegExp(`\\\\${command}\\s*\\{([^{}]*)\\}`, 'g');
  let previous = value;
  let next = previous.replace(pattern, (_match, content: string) => format(content));
  while (next !== previous) {
    previous = next;
    next = previous.replace(pattern, (_match, content: string) => format(content));
  }
  return next;
}

function renderMathTextWithScripts(value: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let textBuffer = '';
  let keyIndex = 0;

  const flushText = () => {
    if (!textBuffer) {
      return;
    }
    nodes.push(textBuffer);
    textBuffer = '';
  };

  for (let index = 0; index < value.length;) {
    const marker = value[index];
    if (marker === '^' || marker === '_') {
      const script = readMathScript(value, index + 1);
      if (script.text) {
        flushText();
        const Tag = marker === '^' ? 'sup' : 'sub';
        nodes.push(
          <Tag
            key={`math-script-${keyIndex}`}
            className={marker === '^' ? 'align-super text-[0.72em] leading-none' : 'align-sub text-[0.72em] leading-none'}
          >
            {renderMathTextWithScripts(script.text)}
          </Tag>,
        );
        keyIndex += 1;
        index = script.nextIndex;
        continue;
      }
    }

    if (marker !== '{' && marker !== '}') {
      textBuffer += marker;
    }
    index += 1;
  }

  flushText();
  return nodes;
}

function readMathScript(value: string, start: number): { text: string; nextIndex: number } {
  let index = start;
  while (value[index] === ' ') {
    index += 1;
  }
  if (value[index] === '{') {
    let depth = 1;
    for (let cursor = index + 1; cursor < value.length; cursor += 1) {
      if (value[cursor] === '{') {
        depth += 1;
      } else if (value[cursor] === '}') {
        depth -= 1;
      }
      if (depth === 0) {
        return {
          text: value.slice(index + 1, cursor),
          nextIndex: cursor + 1,
        };
      }
    }
  }
  return {
    text: value[index] || '',
    nextIndex: Math.min(index + 1, value.length),
  };
}

function readUnclosedMarkdownFence(content: string): { activeFence: string | null; fenceCount: number } {
  let activeFence: string | null = null;
  let fenceCount = 0;
  const parts = content.split(/(\r\n|\n|\r)/);

  for (let index = 0; index < parts.length; index += 2) {
    const marker = readMarkdownFenceMarker(parts[index] || '');
    if (!marker) {
      continue;
    }
    if (!activeFence) {
      activeFence = marker;
      fenceCount += 1;
      continue;
    }
    if (marker[0] === activeFence[0] && marker.length >= activeFence.length) {
      activeFence = null;
      fenceCount += 1;
    }
  }

  return { activeFence, fenceCount };
}

function readMarkdownFenceMarker(line: string): string | null {
  const match = /^(?: {0,3})(`{3,}|~{3,})/.exec(line);
  return match?.[1] || null;
}

function mapMarkdownLinesOutsideCodeFences(
  content: string,
  transform: (line: string) => string,
): string {
  let activeFence: string | null = null;
  const parts = content.split(/(\r\n|\n|\r)/);
  let result = '';

  for (let index = 0; index < parts.length; index += 2) {
    const line = parts[index] || '';
    const newline = parts[index + 1] || '';
    const marker = readMarkdownFenceMarker(line);
    if (marker && !activeFence) {
      activeFence = marker;
      result += line + newline;
      continue;
    }
    if (marker && activeFence && marker[0] === activeFence[0] && marker.length >= activeFence.length) {
      activeFence = null;
      result += line + newline;
      continue;
    }
    result += (activeFence ? line : transform(line)) + newline;
  }

  return result;
}

function escapeRawHtmlTags(line: string): string {
  return line.replace(
    /<!--[\s\S]*?-->|<\/?[A-Za-z][A-Za-z0-9:-]*(?:\s+[^<>]*)?\/?>|<![A-Za-z][^<>]*>/g,
    (token) => (isMarkdownAutolinkToken(token) ? token : escapeHtmlToken(token)),
  );
}

function isMarkdownAutolinkToken(token: string): boolean {
  return /^<https?:\/\//i.test(token)
    || /^<mailto:/i.test(token)
    || /^<[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+>$/i.test(token);
}

function escapeHtmlToken(token: string): string {
  return token
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function readMarkdownTextFromUnknown(value: unknown, depth: number): string | null {
  if (depth > 5 || value === null || value === undefined) {
    return null;
  }

  if (typeof value === 'string') {
    const parsed = parseJsonLikeMarkdownPayload(value);
    if (parsed !== null) {
      const parsedText = readMarkdownTextFromUnknown(parsed, depth + 1);
      if (parsedText !== null) {
        return parsedText;
      }
    }
    return decodeTransportEscapedMarkdown(value);
  }

  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }

  if (Array.isArray(value)) {
    const parts = value
      .map((item) => readMarkdownTextFromUnknown(item, depth + 1))
      .filter((item): item is string => Boolean(item?.trim()));
    return parts.length > 0 ? parts.join('\n') : null;
  }

  if (typeof value !== 'object') {
    return null;
  }

  const record = value as Record<string, unknown>;
  const choiceText = readMarkdownTextFromChoices(record, depth + 1);
  if (choiceText) {
    return choiceText;
  }

  for (const key of [
    'outputText',
    'output_text',
    'textDelta',
    'text_delta',
    'delta',
    'text',
    'content',
    'message',
    'response',
    'data',
    'result',
    'payload',
  ]) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      continue;
    }
    const text = readMarkdownTextFromUnknown(record[key], depth + 1);
    if (text?.trim()) {
      return text;
    }
  }

  return null;
}

function readMarkdownTextFromChoices(record: Record<string, unknown>, depth: number): string | null {
  const choices = record.choices;
  if (!Array.isArray(choices)) {
    return null;
  }
  const parts = choices
    .map((choice) => {
      if (!choice || typeof choice !== 'object') {
        return readMarkdownTextFromUnknown(choice, depth + 1);
      }
      const choiceRecord = choice as Record<string, unknown>;
      return readMarkdownTextFromUnknown(choiceRecord.delta, depth + 1)
        ?? readMarkdownTextFromUnknown(choiceRecord.message, depth + 1)
        ?? readMarkdownTextFromUnknown(choiceRecord.text, depth + 1)
        ?? readMarkdownTextFromUnknown(choiceRecord.content, depth + 1);
    })
    .filter((item): item is string => Boolean(item?.trim()));
  return parts.length > 0 ? parts.join('\n') : null;
}

function parseJsonLikeMarkdownPayload(value: string): unknown | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const first = trimmed[0];
  const last = trimmed[trimmed.length - 1];
  const mayBeJson = (first === '{' && last === '}')
    || (first === '[' && last === ']')
    || (first === '"' && last === '"');
  if (!mayBeJson) {
    return null;
  }
  try {
    return JSON.parse(trimmed);
  } catch {
    return null;
  }
}

function decodeTransportEscapedMarkdown(value: string): string {
  if (!shouldDecodeTransportEscapes(value)) {
    return value;
  }
  return value
    .replace(/\\r\\n/g, '\n')
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\n')
    .replace(/\\t/g, '\t');
}

function normalizeProfessionalMarkdownSyntax(value: string): string {
  return normalizeSoftLineBreaks(
    normalizeInlineMathExpressions(
      normalizeDisplayMathBlocks(value),
    ),
  );
}

function normalizeDisplayMathBlocks(value: string): string {
  let activeFence: string | null = null;
  let activeMath: {
    delimiter: '$$' | '\\[';
    lines: string[];
    original: string;
  } | null = null;
  const parts = value.split(/(\r\n|\n|\r)/);
  let result = '';

  for (let index = 0; index < parts.length; index += 2) {
    const line = parts[index] || '';
    const newline = parts[index + 1] || '';

    if (activeMath) {
      if (isClosingDisplayMathDelimiter(line, activeMath.delimiter)) {
        const original = activeMath.original + line + newline;
        const tex = activeMath.lines.join('\n').trim();
        result += tex ? `${writeDisplayMathFence(tex)}${newline}` : original;
        activeMath = null;
        continue;
      }
      activeMath.lines.push(line);
      activeMath.original += line + newline;
      continue;
    }

    const marker = readMarkdownFenceMarker(line);
    if (marker && !activeFence) {
      activeFence = marker;
      result += line + newline;
      continue;
    }
    if (marker && activeFence && marker[0] === activeFence[0] && marker.length >= activeFence.length) {
      activeFence = null;
      result += line + newline;
      continue;
    }
    if (activeFence) {
      result += line + newline;
      continue;
    }

    const singleLineMath = readSingleLineDisplayMath(line);
    if (singleLineMath !== null) {
      result += `${writeDisplayMathFence(singleLineMath)}${newline}`;
      continue;
    }

    const delimiter = readOpeningDisplayMathDelimiter(line);
    if (delimiter) {
      activeMath = {
        delimiter,
        lines: [],
        original: line + newline,
      };
      continue;
    }

    result += line + newline;
  }

  return result + (activeMath?.original || '');
}

function readOpeningDisplayMathDelimiter(line: string): '$$' | '\\[' | null {
  const trimmed = line.trim();
  if (trimmed === '$$' || trimmed === '\\[') {
    return trimmed;
  }
  return null;
}

function isClosingDisplayMathDelimiter(line: string, delimiter: '$$' | '\\['): boolean {
  const trimmed = line.trim();
  return delimiter === '$$' ? trimmed === '$$' : trimmed === '\\]';
}

function readSingleLineDisplayMath(line: string): string | null {
  const trimmed = line.trim();
  if (trimmed.startsWith('$$') && trimmed.endsWith('$$') && trimmed.length > 4) {
    return trimmed.slice(2, -2).trim() || null;
  }
  if (trimmed.startsWith('\\[') && trimmed.endsWith('\\]') && trimmed.length > 4) {
    return trimmed.slice(2, -2).trim() || null;
  }
  return null;
}

function writeDisplayMathFence(tex: string): string {
  return `\`\`\`${MATH_DISPLAY_LANGUAGE}\n${encodeURIComponent(tex)}\n\`\`\``;
}

function normalizeInlineMathExpressions(value: string): string {
  return mapMarkdownLinesOutsideCodeFences(value, normalizeInlineMathInLine);
}

function normalizeInlineMathInLine(line: string): string {
  let result = '';
  let index = 0;

  while (index < line.length) {
    if (line[index] === '`') {
      const runLength = readBacktickRunLength(line, index);
      const end = findInlineCodeSpanEnd(line, index, runLength);
      if (end >= 0) {
        const endIndex = end + runLength;
        result += line.slice(index, endIndex);
        index = endIndex;
        continue;
      }
    }

    if (line[index] === '\\' && line[index + 1] === '(') {
      const end = findInlineParenMathEnd(line, index + 2);
      if (end > index) {
        const tex = line.slice(index + 2, end).trim();
        if (looksLikeInlineMath(tex)) {
          result += `\`${INLINE_MATH_PREFIX}${encodeURIComponent(tex)}__\``;
          index = end + 2;
          continue;
        }
      }
    }

    if (line[index] === '$' && line[index - 1] !== '\\' && line[index + 1] !== '$') {
      const end = findInlineMathEnd(line, index + 1);
      if (end > index && hasDollarMathDelimiterContext(line, index, end)) {
        const tex = line.slice(index + 1, end).trim();
        if (looksLikeInlineMath(tex)) {
          result += `\`${INLINE_MATH_PREFIX}${encodeURIComponent(tex)}__\``;
          index = end + 1;
          continue;
        }
      }
    }

    result += line[index];
    index += 1;
  }

  return result;
}

function readBacktickRunLength(value: string, index: number): number {
  let cursor = index;
  while (value[cursor] === '`') {
    cursor += 1;
  }
  return cursor - index;
}

function findInlineCodeSpanEnd(value: string, start: number, runLength: number): number {
  return value.indexOf('`'.repeat(runLength), start + runLength);
}

function findInlineMathEnd(value: string, start: number): number {
  for (let index = start; index < value.length; index += 1) {
    if (value[index] === '$' && value[index - 1] !== '\\' && value[index + 1] !== '$') {
      return index;
    }
  }
  return -1;
}

function findInlineParenMathEnd(value: string, start: number): number {
  for (let index = start; index < value.length - 1; index += 1) {
    if (value[index] === '\\' && value[index + 1] === ')') {
      return index;
    }
  }
  return -1;
}

function hasDollarMathDelimiterContext(value: string, start: number, end: number): boolean {
  const before = value[start - 1] || '';
  const first = value[start + 1] || '';
  const last = value[end - 1] || '';
  const after = value[end + 1] || '';
  if (/\s/.test(first) || /\s/.test(last)) {
    return false;
  }
  if (/[A-Za-z0-9]/.test(before) || /[A-Za-z0-9]/.test(after)) {
    return false;
  }
  return true;
}

function looksLikeInlineMath(tex: string): boolean {
  if (!tex || /^\d+(?:[.,]\d+)?$/.test(tex)) {
    return false;
  }
  return /[\\^_{}=+\-*/<>]/.test(tex)
    || /[A-Za-z]\s*[=<>]/.test(tex)
    || /\\[A-Za-z]+/.test(tex);
}

function normalizeSoftLineBreaks(value: string): string {
  let activeFence: string | null = null;
  const parts = value.split(/(\r\n|\n|\r)/);
  let result = '';

  for (let index = 0; index < parts.length; index += 2) {
    const line = parts[index] || '';
    const newline = parts[index + 1] || '';
    const marker = readMarkdownFenceMarker(line);
    if (marker && !activeFence) {
      activeFence = marker;
      result += line + newline;
      continue;
    }
    if (marker && activeFence && marker[0] === activeFence[0] && marker.length >= activeFence.length) {
      activeFence = null;
      result += line + newline;
      continue;
    }
    if (activeFence) {
      result += line + newline;
      continue;
    }

    const nextLine = parts[index + 2] || '';
    result += shouldAppendSoftLineBreak(line, nextLine)
      ? `${trimTrailingHorizontalWhitespace(line)}  ${newline}`
      : line + newline;
  }

  return result;
}

function shouldAppendSoftLineBreak(line: string, nextLine: string): boolean {
  const trimmed = line.trim();
  const nextTrimmed = nextLine.trim();
  if (!trimmed || !nextTrimmed || /(?: {2}|\\)$/.test(line)) {
    return false;
  }
  return !isMarkdownStructuralLine(trimmed) && !isMarkdownStructuralLine(nextTrimmed);
}

function isMarkdownStructuralLine(trimmedLine: string): boolean {
  return /^(?:#{1,6}\s|[-*+]\s|\d{1,9}\.\s|>\s?|`{3,}|~{3,}|\[\^[^\]]+\]:)/.test(trimmedLine)
    || /^\|.*\|$/.test(trimmedLine)
    || /^:?-{3,}:?(?:\s*\|\s*:?-{3,}:?)+$/.test(trimmedLine)
    || /^(?:[-*_]\s*){3,}$/.test(trimmedLine);
}

function normalizeCompactMarkdownSyntax(value: string): string {
  return normalizeCompactOrderedLists(normalizeInlineMarkdownFences(value));
}

function normalizeInlineMarkdownFences(value: string): string {
  let result = '';
  let cursor = 0;
  let activeFence: string | null = null;
  const fencePattern = /(`{3,}|~{3,})/g;
  let match: RegExpExecArray | null;

  while ((match = fencePattern.exec(value)) !== null) {
    const marker = match[0];
    const markerIndex = match.index;
    if (!activeFence && !isFenceOpeningAt(value, markerIndex, marker)) {
      continue;
    }
    if (activeFence && (marker[0] !== activeFence[0] || marker.length < activeFence.length)) {
      continue;
    }

    result += value.slice(cursor, markerIndex);
    const linePrefix = value.slice(findLineStart(value, markerIndex), markerIndex);
    const markerHasLinePrefix = linePrefix.trim().length > 0;
    if (markerHasLinePrefix) {
      result = trimTrailingHorizontalWhitespace(result);
      result += activeFence ? '\n' : '\n\n';
    }

    result += marker;
    cursor = markerIndex + marker.length;

    if (!activeFence) {
      const compactOpening = readCompactOpeningFenceLineRepair(value, cursor, marker);
      if (compactOpening) {
        result += compactOpening.replacement;
        cursor = compactOpening.nextIndex;
      }
      activeFence = marker;
      continue;
    }

    activeFence = null;
    const next = value[cursor];
    if (next && next !== '\n' && next !== '\r') {
      result += '\n\n';
    }
  }

  return result + value.slice(cursor);
}

function isFenceOpeningAt(value: string, markerIndex: number, marker: string): boolean {
  const linePrefix = value.slice(findLineStart(value, markerIndex), markerIndex);
  if (linePrefix.trim().length === 0) {
    return true;
  }
  const lineRemainder = value.slice(markerIndex + marker.length, findLineEnd(value, markerIndex));
  return /^[A-Za-z0-9_+#.-]*[ \t]*$/.test(lineRemainder)
    || readCompactOpeningFenceLineRepair(value, markerIndex + marker.length, marker) !== null;
}

function readCompactOpeningFenceLineRepair(
  value: string,
  startIndex: number,
  marker: string,
): { replacement: string; nextIndex: number } | null {
  const newlineIndex = findLineEnd(value, startIndex);
  const sameLineClosingIndex = value.indexOf(marker, startIndex);
  const lineEnd = sameLineClosingIndex >= 0 && sameLineClosingIndex < newlineIndex
    ? sameLineClosingIndex
    : newlineIndex;
  const lineRemainder = value.slice(startIndex, lineEnd);
  const collapsedLanguage = readCollapsedLanguageFenceRepair(lineRemainder);
  if (collapsedLanguage) {
    return {
      replacement: collapsedLanguage,
      nextIndex: lineEnd,
    };
  }

  const match = /^([A-Za-z0-9_+#.-]+)[ \t]+(\S[\s\S]*)$/.exec(lineRemainder);
  if (match) {
    const [, language, code] = match;
    if (isLikelyFenceLanguage(language) && looksLikeCompactFenceCode(code)) {
      return {
        replacement: `${language}\n${repairCollapsedCodeStatementBreaks(code)}`,
        nextIndex: lineEnd,
      };
    }
  }

  const unlabeledCode = lineRemainder.trimStart();
  if (!looksLikeCompactFenceCode(unlabeledCode)) {
    return null;
  }
  return {
    replacement: `\n${repairCollapsedCodeStatementBreaks(unlabeledCode)}`,
    nextIndex: lineEnd,
  };
}

function readCollapsedLanguageFenceRepair(value: string): string | null {
  const trimmedStartLength = value.length - value.trimStart().length;
  const prefix = value.slice(0, trimmedStartLength);
  const candidate = value.slice(trimmedStartLength);
  const language = readCollapsedFenceLanguage(candidate);
  if (!language) {
    return null;
  }
  const code = candidate.slice(language.length);
  if (!looksLikeCompactFenceCode(code)) {
    return null;
  }
  return `${prefix}${language}\n${repairCollapsedCodeStatementBreaks(code)}`;
}

function readCollapsedFenceLanguage(value: string): string | null {
  const normalized = value.toLowerCase();
  for (const language of COLLAPSED_FENCE_LANGUAGE_PREFIXES) {
    if (normalized.startsWith(language) && looksLikeCodeStart(value.slice(language.length))) {
      return value.slice(0, language.length);
    }
  }
  return null;
}

function isLikelyFenceLanguage(value: string): boolean {
  const normalized = value.toLowerCase();
  if (!/^[a-z][a-z0-9_+#.-]{0,31}$/i.test(value)) {
    return false;
  }
  return ![
    'async',
    'await',
    'class',
    'const',
    'def',
    'export',
    'fn',
    'from',
    'function',
    'import',
    'interface',
    'let',
    'package',
    'private',
    'public',
    'return',
    'select',
    'struct',
    'type',
    'use',
    'var',
  ].includes(normalized);
}

function looksLikeCompactFenceCode(value: string): boolean {
  const trimmed = value.trim();
  return /[{}()[\];=<>]/.test(trimmed)
    || /\b(?:class|const|def|export|fn|from|function|import|interface|let|package|private|public|return|select|struct|type|use|var)\b/i.test(trimmed);
}

function looksLikeCodeStart(value: string): boolean {
  const trimmed = value.trimStart();
  return /^(?:class|const|def|export|fn|from|function|import|interface|let|package|private|public|return|select|struct|type|use|var)\b/i.test(trimmed)
    || /^[{(<[]/.test(trimmed);
}

function repairCollapsedCodeStatementBreaks(value: string): string {
  return value
    .replace(/;[ \t]*(?=(?:class|const|def|export|fn|from|function|import|interface|let|package|private|public|return|select|struct|type|use|var)\b)/gi, ';\n')
    .replace(/\}[ \t]*(?=(?:catch|else|finally)\b)/gi, '}\n');
}

const COLLAPSED_FENCE_LANGUAGE_PREFIXES = [
  'typescript',
  'javascript',
  'python',
  'kotlin',
  'rust',
  'java',
  'tsx',
  'jsx',
  'ts',
  'js',
  'py',
  'rs',
  'go',
  'sql',
] as const;

function findLineStart(value: string, index: number): number {
  const previousNewline = value.lastIndexOf('\n', Math.max(0, index - 1));
  return previousNewline < 0 ? 0 : previousNewline + 1;
}

function findLineEnd(value: string, index: number): number {
  const nextNewline = value.indexOf('\n', index);
  return nextNewline < 0 ? value.length : nextNewline;
}

function trimTrailingHorizontalWhitespace(value: string): string {
  return value.replace(/[ \t]+$/g, '');
}

function normalizeCompactOrderedLists(value: string): string {
  return value
    .replace(/([:：])(?=\d{1,3}\.(?!\d))/g, '$1\n\n')
    .replace(/(^|\n)( {0,3}\d{1,3}\.)(?=\S)/g, '$1$2 ')
    .replace(/(\*\*[^*\n]+?\*\*)(?=[\p{L}\p{N}])/gu, '$1 ');
}

function shouldDecodeTransportEscapes(value: string): boolean {
  if (!/(?:\\r\\n|\\n|\\r)/.test(value)) {
    return false;
  }
  const decoded = value
    .replace(/\\r\\n/g, '\n')
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\n')
    .replace(/\\t/g, '\t');
  return looksLikeMarkdown(decoded) || decoded.split('\n').length >= 3;
}

function looksLikeMarkdown(value: string): boolean {
  if (/(```|~~~)/.test(value)) {
    return true;
  }
  const lines = value.split(/\r\n|\n|\r/);
  return lines.some((line) => /^(?: {0,3})(#{1,6}\s|[-*+]\s|\d+\.\s|>\s|\|.+\|)/.test(line))
    || /\[[^\]]+\]\([^)]+\)/.test(value)
    || /[*_~]{2}[^*_~]+[*_~]{2}/.test(value);
}

function mergeClassNames(...values: Array<string | undefined | false>): string {
  return values.filter(Boolean).join(' ');
}
