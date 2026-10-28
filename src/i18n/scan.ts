// F34: acha texto de tela que ficou no código em vez do catálogo. Usa o parser do
// TypeScript; roda no Node pelo teste `i18n.test.ts`, nunca no app.

import ts from "typescript";

export interface Leftover {
  file: string;
  line: number;
  text: string;
}

// Atributos JSX que nunca são texto para pessoas.
const CODE_ATTRIBUTES = new Set([
  "className", "d", "viewBox", "fill", "stroke", "strokeWidth", "strokeLinecap", "strokeLinejoin", "type", "href", "role",
  "key", "id", "htmlFor", "preload", "decoding", "loading", "accept", "autoComplete", "inputMode", "name", "method",
  "transform", "points", "x", "y", "rx", "cx", "cy", "r", "width", "height", "src", "lang", "dir", "aria-live",
  "aria-current", "aria-modal", "aria-hidden", "aria-labelledby", "aria-describedby", "aria-controls", "aria-haspopup",
]);

// Chamadas cujo texto é código: comandos do Rust, chaves do catálogo, seletores, eventos.
const CODE_CALLS = new Set([
  "invoke", "t", "list", "maybe", "querySelector", "querySelectorAll", "addEventListener", "removeEventListener",
  "getItem", "setItem", "removeItem", "matchMedia", "listen", "createElement", "getPropertyValue", "setProperty",
  "convertFileSrc", "RegExp", "closest", "matches", "require", "import",
]);

// Nomes que não se traduzem: a marca e teclas.
const PROPER = new Set(["My Photos", "Ctrl F", "Esc", "SHA-256", "JSON", "EXIF", "Takeout", "Natural Earth · GeoNames"]);

/** Parece frase para pessoas: palavra com maiúscula, duas palavras ou um acento. */
export function looksLikeText(text: string): boolean {
  const trimmed = text.trim();
  if (!/\p{L}{2}/u.test(trimmed) || PROPER.has(trimmed)) {
    return false;
  }
  if (/^[A-ZÀ-Ý]\p{Ll}+[…:.!?]*$/u.test(trimmed)) {
    return true;
  }
  if (/^[\w.-]+$/.test(trimmed) && !/[À-ÿ]/.test(trimmed)) {
    return false;
  }
  if (/^(#|\/|\.\/|\.\.\/|https?:|[a-z]+:\/\/|M[\d ]|--)/.test(trimmed)) {
    return false;
  }
  return /\p{L}+\s+\p{L}+/u.test(trimmed) || /\d\s+\p{L}{3,}/u.test(trimmed) || /[À-ÿ]/.test(trimmed);
}

function calleeName(node: ts.CallExpression | ts.NewExpression): string | undefined {
  const callee = node.expression;
  if (ts.isIdentifier(callee)) {
    return callee.text;
  }
  if (ts.isPropertyAccessExpression(callee)) {
    return callee.name.text;
  }
  return undefined;
}

function skipped(node: ts.Node): boolean {
  for (let parent = node.parent; parent !== undefined; parent = parent.parent) {
    if (ts.isImportDeclaration(parent) || ts.isExportDeclaration(parent)) {
      return true;
    }
    if (ts.isJsxAttribute(parent)) {
      return CODE_ATTRIBUTES.has(parent.name.getText());
    }
    if ((ts.isCallExpression(parent) || ts.isNewExpression(parent)) && CODE_CALLS.has(calleeName(parent) ?? "")) {
      return true;
    }
    if (ts.isLiteralTypeNode(parent) || ts.isCaseClause(parent)) {
      return true;
    }
    if (ts.isBinaryExpression(parent) && /^(===|!==|==|!=)$/.test(parent.operatorToken.getText())) {
      return true;
    }
    if (ts.isPropertyAssignment(parent) && parent.initializer !== node && !parent.initializer.getText().includes(node.getText())) {
      return true;
    }
    if (ts.isElementAccessExpression(parent) || ts.isPropertyAccessExpression(parent)) {
      return false;
    }
    if (ts.isBlock(parent) || ts.isSourceFile(parent)) {
      return false;
    }
  }
  return false;
}

/** Texto visível ainda escrito no código de `file`. */
export function leftovers(file: string, source: string): Leftover[] {
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const found: Leftover[] = [];
  const report = (node: ts.Node, text: string) => {
    if (looksLikeText(text) && !skipped(node)) {
      const { line } = tree.getLineAndCharacterOfPosition(node.getStart());
      found.push({ file, line: line + 1, text: text.trim() });
    }
  };
  const visit = (node: ts.Node) => {
    if (ts.isJsxText(node)) {
      report(node, node.text);
    } else if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      report(node, node.text);
    } else if (ts.isTemplateExpression(node)) {
      // Num template, qualquer palavra solta ("desde ${data}") já é texto.
      const parts = [node.head.text, ...node.templateSpans.map((span) => span.literal.text)];
      const word = parts.find(
        (part) => (/(^|\s)\p{L}{2,}(\s|$)/u.test(part) && !/^[\w.:-]*$/.test(part)) || /^\s+\p{L}+\s+$/u.test(part),
      );
      if (word !== undefined && !skipped(node)) {
        const { line } = tree.getLineAndCharacterOfPosition(node.getStart());
        found.push({ file, line: line + 1, text: parts.join("…").trim() });
      } else {
        report(node, parts.join(" "));
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(tree);
  return found;
}
