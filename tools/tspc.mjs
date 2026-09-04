import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";

const METHOD_NAMES = new Set([
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
  "ANY",
]);

const SOURCE_EXTENSIONS = new Set([".tsp", ".ts", ".tsx", ".js", ".jsx"]);

export class TspCompileError extends Error {
  constructor(code, message, fileName = undefined) {
    super(message);
    this.name = "TspCompileError";
    this.code = code;
    this.fileName = fileName;
  }
}

function isExported(node) {
  return node.modifiers?.some(
    (modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword,
  ) ?? false;
}

function exportedNames(sourceFile) {
  const names = new Set();

  for (const statement of sourceFile.statements) {
    if (!isExported(statement)) continue;

    if (statement.name?.text) {
      names.add(statement.name.text);
    }

    if (ts.isVariableStatement(statement)) {
      for (const declaration of statement.declarationList.declarations) {
        if (ts.isIdentifier(declaration.name)) names.add(declaration.name.text);
      }
    }

    if (ts.isExportDeclaration(statement) && statement.exportClause) {
      if (ts.isNamedExports(statement.exportClause)) {
        for (const element of statement.exportClause.elements) {
          names.add((element.name ?? element.propertyName).text);
        }
      }
    }
  }

  if (sourceFile.statements.some((statement) => ts.isExportAssignment(statement))) {
    names.add("default");
  }

  return [...names].sort();
}

function routeFromFile(relativeName) {
  const segments = relativeName
    .replaceAll("\\", "/")
    .replace(/\.(tsp|tsx?|jsx?)$/, "")
    .split("/");

  const file = segments.pop();
  if (file === "index") segments.push("");
  else segments.push(file);

  const parameters = [];
  const routeSegments = segments.map((segment) => {
    if (segment === "") return "";
    if (/^\[\.\.\.[A-Za-z_$][\w$]*\]$/.test(segment)) {
      parameters.push(segment.slice(4, -1));
      return "*";
    }
    if (/^\[[A-Za-z_$][\w$]*\]$/.test(segment)) {
      parameters.push(segment.slice(1, -1));
      return `:${segment.slice(1, -1)}`;
    }
    if (segment.startsWith("[") || segment.endsWith("]")) {
      throw new TspCompileError("TSP1002", `unsupported route segment: ${segment}`);
    }
    return segment;
  });

  const route = `/${routeSegments.filter((segment) => segment !== "").join("/")}`;
  return {
    path: route === "/" ? "/" : route.replace(/\/{2,}/g, "/"),
    parameters,
  };
}

function formatDiagnostic(diagnostic) {
  return ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");
}

function validateSource(fileName, source) {
  const sourceFile = ts.createSourceFile(
    fileName,
    source,
    ts.ScriptTarget.Latest,
    true,
    fileName.endsWith(".tsx") || fileName.endsWith(".jsx") || fileName.endsWith(".tsp")
      ? ts.ScriptKind.TSX
      : ts.ScriptKind.TS,
  );

  const parseErrors = sourceFile.parseDiagnostics;
  if (parseErrors.length > 0) {
    throw new TspCompileError("TSP3001", formatDiagnostic(parseErrors[0]), fileName);
  }

  for (const statement of sourceFile.statements) {
    if (!ts.isImportDeclaration(statement)) continue;
    const specifier = statement.moduleSpecifier.text;
    if (typeof specifier === "string" && /\.tsp(?:$|[?#])/.test(specifier)) {
      throw new TspCompileError(
        "TSP1005",
        "a .tsp route must not import another .tsp route",
        fileName,
      );
    }
  }

  const exports = exportedNames(sourceFile);
  const handlers = exports.filter((name) => METHOD_NAMES.has(name));
  if (fileName.endsWith(".tsp")) {
    if (exports.includes("default")) {
      throw new TspCompileError("TSP3001", "route modules cannot have a default export", fileName);
    }
    const unknownFunctions = sourceFile.statements
      .filter((statement) => isExported(statement) && ts.isFunctionDeclaration(statement))
      .map((statement) => statement.name?.text)
      .filter((name) => name && !METHOD_NAMES.has(name));
    if (unknownFunctions.length > 0) {
      throw new TspCompileError(
        "TSP3001",
        `unsupported exported route function: ${unknownFunctions[0]}`,
        fileName,
      );
    }
    if (handlers.length === 0) {
      throw new TspCompileError("TSP3001", "route must export at least one HTTP handler", fileName);
    }
  }

  return { sourceFile, exports, handlers };
}

function compileSource(fileName, source) {
  const checked = validateSource(fileName, source);
  // TypeScript does not know the TSP extension. Use a virtual TSX name so
  // the compiler selects the same parser for `.tsp` files as for `.tsx`.
  const transpileFileName = fileName.endsWith(".tsp")
    ? `${fileName.slice(0, -4)}.tsx`
    : fileName;
  const result = ts.transpileModule(source, {
    fileName: transpileFileName,
    reportDiagnostics: true,
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      // TSP supplies the module registry. CommonJS output gives the bundle a
      // small, explicit `(module, exports, require)` boundary that does not
      // depend on JSC's optional module loader.
      module: ts.ModuleKind.CommonJS,
      jsx: ts.JsxEmit.React,
      jsxFactory: "__tsp_jsx",
      jsxFragmentFactory: "__tsp_fragment",
      sourceMap: true,
      inlineSources: true,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
  });

  const diagnostics = result.diagnostics ?? [];
  if (diagnostics.length > 0) {
    throw new TspCompileError("TSP3001", formatDiagnostic(diagnostics[0]), fileName);
  }

  const prelude = [
    "const __tsp_jsx = globalThis.__tsp_jsx;",
    "const __tsp_fragment = globalThis.__tsp_fragment;",
    "",
  ].join("\n");

  return {
    ...checked,
    code: `${prelude}${result.outputText}`,
  };
}

function outputName(relativeName) {
  return relativeName.replace(/\.(tsp|tsx?|jsx?)$/, ".js").replaceAll("\\", "/");
}

function importCandidates(base) {
  return [
    base,
    `${base}.tsp`,
    `${base}.tsx`,
    `${base}.ts`,
    `${base}.jsx`,
    `${base}.js`,
    `${base}/index.tsp`,
    `${base}/index.tsx`,
    `${base}/index.ts`,
    `${base}/index.jsx`,
    `${base}/index.js`,
  ];
}

function canonicalImport(fromSource, specifier, sourceToOutput) {
  if (specifier.startsWith("tsp:")) return specifier;
  if (!specifier.startsWith(".")) {
    throw new TspCompileError("TSP1006", `unsupported external module: ${specifier}`, fromSource);
  }
  const base = path.posix.normalize(path.posix.join(path.posix.dirname(fromSource), specifier));
  for (const candidate of importCandidates(base)) {
    const output = sourceToOutput.get(candidate);
    if (output) return output;
  }
  throw new TspCompileError("TSP1006", `cannot resolve module: ${specifier}`, fromSource);
}

function rewriteModuleRequires(code, sourceName, sourceToOutput) {
  const withRuntimeRequire = code.replace(/\brequire\s*\(/g, "__tsp_require(");
  return withRuntimeRequire.replace(/__tsp_require\((['"])([^'"]+)\1\)/g, (_match, quote, specifier) => {
    const canonical = canonicalImport(sourceName, specifier, sourceToOutput);
    return `__tsp_require(${quote}${canonical}${quote})`;
  });
}

function renderBundle(modules, routes) {
  const definitions = modules
    .map(
      (module) =>
        `${JSON.stringify(module.output)}: (module, exports, __tsp_require) => {\n${module.code}\n}`,
    )
    .join(",\n");
  const registrations = routes
    .map((route) => `__tsp_routes[${JSON.stringify(route.path)}] = __tsp_load(${JSON.stringify(route.output)});`)
    .join("\n");
  return `(function () {
  "use strict";
  const __tsp_definitions = {
${definitions}
  };
  const __tsp_cache = Object.create(null);
  const __tsp_builtins = globalThis.__tsp_builtin_modules || Object.create(null);
  function __tsp_load(id) {
    if (id.startsWith("tsp:")) {
      const builtin = __tsp_builtins[id];
      if (!builtin) throw new Error("TSP builtin module is not installed: " + id);
      return builtin;
    }
    if (__tsp_cache[id]) return __tsp_cache[id].exports;
    const definition = __tsp_definitions[id];
    if (!definition) throw new Error("TSP module is not in the bundle: " + id);
    const module = { exports: {} };
    __tsp_cache[id] = module;
    definition(module, module.exports, __tsp_load);
    return module.exports;
  }
  const __tsp_routes = Object.create(null);
${registrations}
  globalThis.__tsp_routes = __tsp_routes;
})();
`;
}

function walkSources(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      if (entry.name.startsWith(".")) continue;
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (SOURCE_EXTENSIONS.has(path.extname(entry.name))) files.push(absolute);
    }
  };
  visit(root);
  return files.sort();
}

export function compileProject({ root, out }) {
  const rootDirectory = path.resolve(root);
  const outputDirectory = path.resolve(out);
  if (!fs.existsSync(rootDirectory) || !fs.statSync(rootDirectory).isDirectory()) {
    throw new TspCompileError("TSP1001", `source directory does not exist: ${rootDirectory}`);
  }

  const routes = [];
  const modules = [];
  const compiledModules = [];
  const sourceToOutput = new Map();
  for (const fileName of walkSources(rootDirectory)) {
    const relativeName = path.relative(rootDirectory, fileName);
    const source = fs.readFileSync(fileName, "utf8");
    const compiled = compileSource(fileName, source);
    const normalizedSource = relativeName.replaceAll("\\", "/");
    const relativeOutput = outputName(relativeName);
    if ([...sourceToOutput.values()].includes(relativeOutput)) {
      throw new TspCompileError("TSP1007", `multiple sources emit ${relativeOutput}`, fileName);
    }
    sourceToOutput.set(normalizedSource, relativeOutput);
    compiledModules.push({ source: normalizedSource, output: relativeOutput, ...compiled });

    const item = {
      source: normalizedSource,
      output: relativeOutput,
      exports: compiled.exports,
    };
    if (fileName.endsWith(".tsp")) {
      const route = routeFromFile(relativeName);
      routes.push({
        ...item,
        path: route.path,
        parameters: route.parameters,
        methods: compiled.handlers,
      });
    } else {
      modules.push(item);
    }
  }

  for (const module of compiledModules) {
    module.code = rewriteModuleRequires(module.code, module.source, sourceToOutput);
    const outputFile = path.join(outputDirectory, module.output);
    fs.mkdirSync(path.dirname(outputFile), { recursive: true });
    fs.writeFileSync(outputFile, module.code, "utf8");
  }

  routes.sort((a, b) => a.path.localeCompare(b.path));
  const bundleName = "bundle.js";
  fs.mkdirSync(outputDirectory, { recursive: true });
  fs.writeFileSync(path.join(outputDirectory, bundleName), renderBundle(compiledModules, routes), "utf8");
  const manifest = {
    version: 1,
    compiler: "tspc-typescript-frontend",
    sourceRoot: rootDirectory,
    bundle: bundleName,
    routes,
    modules,
  };
  fs.mkdirSync(outputDirectory, { recursive: true });
  fs.writeFileSync(
    path.join(outputDirectory, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
    "utf8",
  );
  return manifest;
}

function parseArgs(argv) {
  const args = [...argv];
  const command = args.shift() ?? "help";
  const values = { command, root: "pages", out: ".tsp-build" };
  while (args.length > 0) {
    const flag = args.shift();
    if (flag === "--root") values.root = args.shift();
    else if (flag === "--out") values.out = args.shift();
    else throw new TspCompileError("TSP1000", `unknown option: ${flag}`);
  }
  return values;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArgs(process.argv.slice(2));
    if (options.command === "compile" || options.command === "check") {
      const manifest = compileProject(options);
      if (options.command === "check") {
        fs.rmSync(path.resolve(options.out), { recursive: true, force: true });
      }
      process.stdout.write(`${JSON.stringify({ routes: manifest.routes.length, modules: manifest.modules.length })}\n`);
    } else {
      process.stdout.write("Usage: node tools/tspc.mjs <compile|check> [--root DIR] [--out DIR]\n");
      process.exitCode = 1;
    }
  } catch (error) {
    const prefix = error.fileName ? `${error.fileName}: ` : "";
    process.stderr.write(`[${error.code ?? "TSP3001"}] ${prefix}${error.message}\n`);
    process.exitCode = 1;
  }
}
