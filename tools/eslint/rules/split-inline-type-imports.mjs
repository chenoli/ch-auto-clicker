import { ESLintUtils } from "@typescript-eslint/utils";

const createRule = ESLintUtils.RuleCreator((name) => `internal-rule://${name}`);

function specifierText(specifier) {
  const imported = specifier.imported.name ?? specifier.imported.value;
  return specifier.local.name === imported ? imported : `${imported} as ${specifier.local.name}`;
}

export const splitInlineTypeImports = createRule({
  name: "split-inline-type-imports",
  meta: {
    type: "suggestion",
    docs: {
      description:
        "Require `import type { ... }` as its own statement instead of an inline `type` specifier mixed with value imports.",
    },
    fixable: "code",
    schema: [],
    messages: {
      splitTypeImport: "Type-only specifiers must be in a separate `import type` statement.",
    },
  },
  defaultOptions: [],
  create(context) {
    return {
      ImportDeclaration(node) {
        if (node.importKind === "type") return;

        const typeSpecifiers = node.specifiers.filter(
          (specifier) => specifier.type === "ImportSpecifier" && specifier.importKind === "type",
        );
        if (typeSpecifiers.length === 0) return;

        const valueSpecifiers = node.specifiers.filter((specifier) => !typeSpecifiers.includes(specifier));
        if (valueSpecifiers.length === 0) return;

        context.report({
          node,
          messageId: "splitTypeImport",
          fix(fixer) {
            const source = context.sourceCode.getText(node.source);

            const defaultSpec = valueSpecifiers.find((specifier) => specifier.type === "ImportDefaultSpecifier");
            const namespaceSpec = valueSpecifiers.find((specifier) => specifier.type === "ImportNamespaceSpecifier");
            const namedValueSpecs = valueSpecifiers.filter((specifier) => specifier.type === "ImportSpecifier");

            const valueParts = [];
            if (defaultSpec) valueParts.push(defaultSpec.local.name);
            if (namespaceSpec) valueParts.push(`* as ${namespaceSpec.local.name}`);
            if (namedValueSpecs.length > 0) {
              valueParts.push(`{ ${namedValueSpecs.map(specifierText).join(", ")} }`);
            }

            const valueImport = `import ${valueParts.join(", ")} from ${source};`;
            const typeImport = `import type { ${typeSpecifiers.map(specifierText).join(", ")} } from ${source};`;

            return fixer.replaceText(node, `${valueImport}\n${typeImport}`);
          },
        });
      },
    };
  },
});

export default splitInlineTypeImports;
