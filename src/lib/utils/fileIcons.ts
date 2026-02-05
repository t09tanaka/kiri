/**
 * File type icon utilities
 * Returns appropriate icon data based on file extension
 */

export interface FileIconInfo {
  icon: string; // SVG path or identifier
  color: string;
}

// Color palette for file icons (Ethereal Mist theme)
const iconColors = {
  // Core languages
  typescript: '#64d2ff', // Cyan accent
  javascript: '#ffd60a', // Bright yellow
  json: '#da8fff', // Light purple
  html: '#ff6961', // Coral red
  css: '#70d7ff', // Light cyan
  svelte: '#ff6b4a', // Svelte orange
  rust: '#f29668', // Warm rust
  markdown: '#5ac8fa', // Sky blue
  image: '#bf5af2', // Purple accent
  git: '#ff453a', // Bright red
  config: '#8090a0', // Muted gray-blue
  text: '#8b949e', // Text gray
  binary: '#6e7681', // Dark gray
  folder: '#64d2ff', // Cyan accent
  folderOpen: '#70d7ff', // Light cyan
  python: '#3776ab', // Python blue
  go: '#00add8', // Go cyan
  vue: '#42b883', // Vue green
  ruby: '#cc342d', // Ruby red
  php: '#777bb4', // PHP purple
  java: '#ea2d2e', // Java red
  kotlin: '#7f52ff', // Kotlin purple
  swift: '#fa7343', // Swift orange
  csharp: '#239120', // C# green
  cpp: '#00599c', // C++ blue
  c: '#555555', // C gray
  graphql: '#e535ab', // GraphQL pink
  prisma: '#2d3748', // Prisma dark
  astro: '#ff5a03', // Astro orange

  // Additional languages
  dart: '#00b4ab', // Dart teal
  scala: '#dc322f', // Scala red
  elixir: '#6e4a7e', // Elixir purple
  erlang: '#a90533', // Erlang red
  haskell: '#5e5086', // Haskell purple
  lua: '#000080', // Lua navy
  perl: '#39457e', // Perl blue
  r: '#276dc3', // R blue
  julia: '#9558b2', // Julia purple
  zig: '#f7a41d', // Zig orange
  nim: '#ffc200', // Nim yellow
  elm: '#60b5cc', // Elm cyan
  clojure: '#5881d8', // Clojure blue
  fsharp: '#b845fc', // F# purple
  ocaml: '#ec6813', // OCaml orange
  rescript: '#e84f4f', // ReScript red
  crystal: '#000000', // Crystal black
  coffeescript: '#28334c', // CoffeeScript dark blue

  // Templates
  pug: '#a86454', // Pug brown
  ejs: '#b4ca65', // EJS green
  handlebars: '#f0772b', // Handlebars orange
  nunjucks: '#1c4913', // Nunjucks green
  liquid: '#168f48', // Liquid green
  haml: '#ece2a9', // Haml cream
  mustache: '#724b3b', // Mustache brown
  marko: '#35a0da', // Marko blue

  // Infrastructure
  terraform: '#7b42bc', // Terraform purple
  protobuf: '#3d85c6', // Protocol Buffers blue
  ini: '#d1dbe0', // INI gray
  nginx: '#009639', // NGINX green

  // Data
  csv: '#89d051', // CSV green
  diff: '#41535b', // Diff gray
  wasm: '#654ff0', // WASM purple

  // Build tools
  gradle: '#02303a', // Gradle dark
  maven: '#c71a36', // Maven red
  webpack: '#8dd6f9', // Webpack blue
  babel: '#f9dc3e', // Babel yellow
  jest: '#c21325', // Jest red
  rollup: '#ff3333', // Rollup red
  esbuild: '#ffcf00', // esbuild yellow
};

// File extension to icon mapping
const extensionMap: Record<string, { type: string; color: string }> = {
  // TypeScript
  ts: { type: 'typescript', color: iconColors.typescript },
  tsx: { type: 'typescript-react', color: iconColors.typescript },
  'd.ts': { type: 'typescript-def', color: iconColors.typescript },

  // JavaScript
  js: { type: 'javascript', color: iconColors.javascript },
  jsx: { type: 'javascript-react', color: iconColors.javascript },
  mjs: { type: 'javascript', color: iconColors.javascript },
  cjs: { type: 'javascript', color: iconColors.javascript },

  // Web
  html: { type: 'html', color: iconColors.html },
  htm: { type: 'html', color: iconColors.html },
  css: { type: 'css', color: iconColors.css },
  scss: { type: 'scss', color: '#c6538c' },
  sass: { type: 'sass', color: '#c6538c' },
  less: { type: 'less', color: '#1d365d' },

  // Svelte
  svelte: { type: 'svelte', color: iconColors.svelte },

  // Rust
  rs: { type: 'rust', color: iconColors.rust },
  toml: { type: 'toml', color: iconColors.config },

  // Data
  json: { type: 'json', color: iconColors.json },
  yaml: { type: 'yaml', color: '#cb171e' },
  yml: { type: 'yaml', color: '#cb171e' },
  xml: { type: 'xml', color: '#e37933' },

  // Documentation
  md: { type: 'markdown', color: iconColors.markdown },
  mdx: { type: 'markdown', color: iconColors.markdown },
  txt: { type: 'text', color: iconColors.text },

  // Images
  png: { type: 'image', color: iconColors.image },
  jpg: { type: 'image', color: iconColors.image },
  jpeg: { type: 'image', color: iconColors.image },
  gif: { type: 'image', color: iconColors.image },
  svg: { type: 'svg', color: '#ffb13b' },
  ico: { type: 'image', color: iconColors.image },
  webp: { type: 'image', color: iconColors.image },

  // Git
  gitignore: { type: 'git', color: iconColors.git },
  gitattributes: { type: 'git', color: iconColors.git },

  // Config
  env: { type: 'env', color: '#ecd53f' },
  'env.local': { type: 'env', color: '#ecd53f' },
  'env.development': { type: 'env', color: '#ecd53f' },
  'env.production': { type: 'env', color: '#ecd53f' },
  eslintrc: { type: 'eslint', color: '#4b32c3' },
  prettierrc: { type: 'prettier', color: '#56b3b4' },

  // Lock files
  lock: { type: 'lock', color: iconColors.config },

  // Shell
  sh: { type: 'shell', color: '#89e051' },
  bash: { type: 'shell', color: '#89e051' },
  zsh: { type: 'shell', color: '#89e051' },

  // Other
  log: { type: 'log', color: iconColors.text },
  sql: { type: 'database', color: '#dad8d8' },

  // Python
  py: { type: 'python', color: iconColors.python },
  pyw: { type: 'python', color: iconColors.python },
  pyx: { type: 'python', color: iconColors.python },
  ipynb: { type: 'jupyter', color: '#f37626' },

  // Go
  go: { type: 'go', color: iconColors.go },
  mod: { type: 'go-mod', color: iconColors.go },
  sum: { type: 'go-sum', color: iconColors.go },

  // Vue
  vue: { type: 'vue', color: iconColors.vue },

  // Ruby
  rb: { type: 'ruby', color: iconColors.ruby },
  erb: { type: 'erb', color: iconColors.ruby },
  rake: { type: 'rake', color: iconColors.ruby },

  // PHP
  php: { type: 'php', color: iconColors.php },

  // Java/Kotlin
  java: { type: 'java', color: iconColors.java },
  kt: { type: 'kotlin', color: iconColors.kotlin },
  kts: { type: 'kotlin', color: iconColors.kotlin },

  // Swift
  swift: { type: 'swift', color: iconColors.swift },

  // C/C++/C#
  c: { type: 'c', color: iconColors.c },
  h: { type: 'c-header', color: iconColors.c },
  cpp: { type: 'cpp', color: iconColors.cpp },
  hpp: { type: 'cpp-header', color: iconColors.cpp },
  cc: { type: 'cpp', color: iconColors.cpp },
  cs: { type: 'csharp', color: iconColors.csharp },

  // GraphQL
  graphql: { type: 'graphql', color: iconColors.graphql },
  gql: { type: 'graphql', color: iconColors.graphql },

  // Prisma
  prisma: { type: 'prisma', color: iconColors.prisma },

  // Astro
  astro: { type: 'astro', color: iconColors.astro },

  // Makefile
  makefile: { type: 'makefile', color: '#6d8086' },
  make: { type: 'makefile', color: '#6d8086' },

  // Dart
  dart: { type: 'dart', color: iconColors.dart },

  // Scala
  scala: { type: 'scala', color: iconColors.scala },
  sc: { type: 'scala', color: iconColors.scala },

  // Elixir
  ex: { type: 'elixir', color: iconColors.elixir },
  exs: { type: 'elixir', color: iconColors.elixir },
  heex: { type: 'elixir', color: iconColors.elixir },

  // Erlang
  erl: { type: 'erlang', color: iconColors.erlang },
  hrl: { type: 'erlang', color: iconColors.erlang },

  // Haskell
  hs: { type: 'haskell', color: iconColors.haskell },
  lhs: { type: 'haskell', color: iconColors.haskell },

  // Lua
  lua: { type: 'lua', color: iconColors.lua },

  // Perl
  pl: { type: 'perl', color: iconColors.perl },
  pm: { type: 'perl', color: iconColors.perl },

  // R
  r: { type: 'r', color: iconColors.r },
  R: { type: 'r', color: iconColors.r },
  rmd: { type: 'rmarkdown', color: iconColors.r },
  Rmd: { type: 'rmarkdown', color: iconColors.r },

  // Julia
  jl: { type: 'julia', color: iconColors.julia },

  // Zig
  zig: { type: 'zig', color: iconColors.zig },

  // Nim
  nim: { type: 'nim', color: iconColors.nim },
  nims: { type: 'nim', color: iconColors.nim },

  // Elm
  elm: { type: 'elm', color: iconColors.elm },

  // Clojure
  clj: { type: 'clojure', color: iconColors.clojure },
  cljs: { type: 'clojure', color: iconColors.clojure },
  cljc: { type: 'clojure', color: iconColors.clojure },
  edn: { type: 'clojure', color: iconColors.clojure },

  // F#
  fs: { type: 'fsharp', color: iconColors.fsharp },
  fsi: { type: 'fsharp', color: iconColors.fsharp },
  fsx: { type: 'fsharp', color: iconColors.fsharp },
  fsproj: { type: 'fsharp', color: iconColors.fsharp },

  // OCaml
  ml: { type: 'ocaml', color: iconColors.ocaml },
  mli: { type: 'ocaml', color: iconColors.ocaml },

  // ReScript / Reason
  res: { type: 'rescript', color: iconColors.rescript },
  resi: { type: 'rescript', color: iconColors.rescript },
  re: { type: 'reason', color: iconColors.rescript },
  rei: { type: 'reason', color: iconColors.rescript },

  // Crystal
  cr: { type: 'crystal', color: iconColors.crystal },

  // CoffeeScript
  coffee: { type: 'coffeescript', color: iconColors.coffeescript },
  litcoffee: { type: 'coffeescript', color: iconColors.coffeescript },

  // Templates
  pug: { type: 'pug', color: iconColors.pug },
  jade: { type: 'pug', color: iconColors.pug },
  ejs: { type: 'ejs', color: iconColors.ejs },
  hbs: { type: 'handlebars', color: iconColors.handlebars },
  handlebars: { type: 'handlebars', color: iconColors.handlebars },
  njk: { type: 'nunjucks', color: iconColors.nunjucks },
  nunjucks: { type: 'nunjucks', color: iconColors.nunjucks },
  liquid: { type: 'liquid', color: iconColors.liquid },
  haml: { type: 'haml', color: iconColors.haml },
  slim: { type: 'slim', color: '#2b5b84' },
  mustache: { type: 'mustache', color: iconColors.mustache },
  marko: { type: 'marko', color: iconColors.marko },
  twig: { type: 'twig', color: '#bcc82a' },

  // Infrastructure / DevOps
  tf: { type: 'terraform', color: iconColors.terraform },
  tfvars: { type: 'terraform', color: iconColors.terraform },
  hcl: { type: 'terraform', color: iconColors.terraform },
  proto: { type: 'protobuf', color: iconColors.protobuf },
  ini: { type: 'ini', color: iconColors.ini },
  conf: { type: 'conf', color: iconColors.config },
  cfg: { type: 'conf', color: iconColors.config },

  // Data formats
  csv: { type: 'csv', color: iconColors.csv },
  tsv: { type: 'tsv', color: iconColors.csv },

  // Diff / Patch
  diff: { type: 'diff', color: iconColors.diff },
  patch: { type: 'diff', color: iconColors.diff },

  // WebAssembly
  wasm: { type: 'wasm', color: iconColors.wasm },
  wat: { type: 'wasm', color: iconColors.wasm },

  // Fonts
  woff: { type: 'font', color: '#c93939' },
  woff2: { type: 'font', color: '#c93939' },
  ttf: { type: 'font', color: '#c93939' },
  otf: { type: 'font', color: '#c93939' },
  eot: { type: 'font', color: '#c93939' },

  // Audio/Video
  mp3: { type: 'audio', color: '#8e44ad' },
  wav: { type: 'audio', color: '#8e44ad' },
  ogg: { type: 'audio', color: '#8e44ad' },
  mp4: { type: 'video', color: '#e74c3c' },
  webm: { type: 'video', color: '#e74c3c' },
  mov: { type: 'video', color: '#e74c3c' },

  // Archive
  zip: { type: 'archive', color: '#e09600' },
  tar: { type: 'archive', color: '#e09600' },
  gz: { type: 'archive', color: '#e09600' },
  rar: { type: 'archive', color: '#e09600' },
  '7z': { type: 'archive', color: '#e09600' },

  // PDF/Docs
  pdf: { type: 'pdf', color: '#ff0000' },
  doc: { type: 'word', color: '#2b579a' },
  docx: { type: 'word', color: '#2b579a' },
  xls: { type: 'excel', color: '#217346' },
  xlsx: { type: 'excel', color: '#217346' },
  ppt: { type: 'powerpoint', color: '#d24726' },
  pptx: { type: 'powerpoint', color: '#d24726' },
};

// Special filename mappings
const filenameMap: Record<string, { type: string; color: string }> = {
  'package.json': { type: 'npm', color: '#cb3837' },
  'package-lock.json': { type: 'npm-lock', color: '#cb3837' },
  'tsconfig.json': { type: 'tsconfig', color: iconColors.typescript },
  'vite.config.ts': { type: 'vite', color: '#646cff' },
  'vite.config.js': { type: 'vite', color: '#646cff' },
  'svelte.config.js': { type: 'svelte', color: iconColors.svelte },
  'tailwind.config.js': { type: 'tailwind', color: '#38bdf8' },
  'tailwind.config.ts': { type: 'tailwind', color: '#38bdf8' },
  '.gitignore': { type: 'git', color: iconColors.git },
  '.gitattributes': { type: 'git', color: iconColors.git },
  'README.md': { type: 'readme', color: '#519aba' },
  LICENSE: { type: 'license', color: '#d4af37' },
  'Cargo.toml': { type: 'cargo', color: iconColors.rust },
  'Cargo.lock': { type: 'cargo-lock', color: iconColors.rust },
  '.env': { type: 'env', color: '#ecd53f' },
  '.env.local': { type: 'env', color: '#ecd53f' },
  '.eslintrc.js': { type: 'eslint', color: '#4b32c3' },
  '.eslintrc.json': { type: 'eslint', color: '#4b32c3' },
  '.prettierrc': { type: 'prettier', color: '#56b3b4' },
  Dockerfile: { type: 'docker', color: '#2496ed' },
  'docker-compose.yml': { type: 'docker', color: '#2496ed' },
  'docker-compose.yaml': { type: 'docker', color: '#2496ed' },

  // Python
  'requirements.txt': { type: 'pip', color: iconColors.python },
  'setup.py': { type: 'python-setup', color: iconColors.python },
  'pyproject.toml': { type: 'python-project', color: iconColors.python },
  Pipfile: { type: 'pipenv', color: iconColors.python },
  'Pipfile.lock': { type: 'pipenv-lock', color: iconColors.python },

  // Go
  'go.mod': { type: 'go-mod', color: iconColors.go },
  'go.sum': { type: 'go-sum', color: iconColors.go },

  // Ruby
  Gemfile: { type: 'gemfile', color: iconColors.ruby },
  'Gemfile.lock': { type: 'gemfile-lock', color: iconColors.ruby },
  Rakefile: { type: 'rake', color: iconColors.ruby },

  // Prisma
  'schema.prisma': { type: 'prisma', color: iconColors.prisma },

  // GraphQL
  'schema.graphql': { type: 'graphql', color: iconColors.graphql },

  // Build
  Makefile: { type: 'makefile', color: '#6d8086' },
  'CMakeLists.txt': { type: 'cmake', color: '#064f8c' },

  // Vercel/Next
  'vercel.json': { type: 'vercel', color: '#000000' },
  'next.config.js': { type: 'next', color: '#000000' },
  'next.config.mjs': { type: 'next', color: '#000000' },
  'next.config.ts': { type: 'next', color: '#000000' },

  // Nuxt
  'nuxt.config.js': { type: 'nuxt', color: '#00dc82' },
  'nuxt.config.ts': { type: 'nuxt', color: '#00dc82' },

  // Astro
  'astro.config.mjs': { type: 'astro', color: iconColors.astro },
  'astro.config.ts': { type: 'astro', color: iconColors.astro },

  // Dart / Flutter
  'pubspec.yaml': { type: 'dart', color: iconColors.dart },
  'pubspec.lock': { type: 'dart', color: iconColors.dart },
  'analysis_options.yaml': { type: 'dart', color: iconColors.dart },

  // Gradle
  'build.gradle': { type: 'gradle', color: iconColors.gradle },
  'build.gradle.kts': { type: 'gradle', color: iconColors.gradle },
  'settings.gradle': { type: 'gradle', color: iconColors.gradle },
  'settings.gradle.kts': { type: 'gradle', color: iconColors.gradle },
  'gradle.properties': { type: 'gradle', color: iconColors.gradle },

  // Maven
  'pom.xml': { type: 'maven', color: iconColors.maven },

  // Build tools
  '.babelrc': { type: 'babel', color: iconColors.babel },
  '.babelrc.json': { type: 'babel', color: iconColors.babel },
  'babel.config.js': { type: 'babel', color: iconColors.babel },
  'babel.config.json': { type: 'babel', color: iconColors.babel },
  'webpack.config.js': { type: 'webpack', color: iconColors.webpack },
  'webpack.config.ts': { type: 'webpack', color: iconColors.webpack },
  'rollup.config.js': { type: 'rollup', color: iconColors.rollup },
  'rollup.config.ts': { type: 'rollup', color: iconColors.rollup },
  'rollup.config.mjs': { type: 'rollup', color: iconColors.rollup },
  'esbuild.config.js': { type: 'esbuild', color: iconColors.esbuild },
  'esbuild.config.mjs': { type: 'esbuild', color: iconColors.esbuild },

  // Testing
  'jest.config.js': { type: 'jest', color: iconColors.jest },
  'jest.config.ts': { type: 'jest', color: iconColors.jest },
  'jest.config.mjs': { type: 'jest', color: iconColors.jest },
  'vitest.config.ts': { type: 'vitest', color: '#729b1b' },
  'vitest.config.js': { type: 'vitest', color: '#729b1b' },
  'playwright.config.ts': { type: 'playwright', color: '#2ead33' },
  'playwright.config.js': { type: 'playwright', color: '#2ead33' },
  'cypress.config.ts': { type: 'cypress', color: '#69d3a7' },
  'cypress.config.js': { type: 'cypress', color: '#69d3a7' },
  'pytest.ini': { type: 'pytest', color: iconColors.python },
  'tox.ini': { type: 'python', color: iconColors.python },
  '.flake8': { type: 'python', color: iconColors.python },
  '.pylintrc': { type: 'python', color: iconColors.python },

  // Terraform
  'main.tf': { type: 'terraform', color: iconColors.terraform },
  'variables.tf': { type: 'terraform', color: iconColors.terraform },
  'outputs.tf': { type: 'terraform', color: iconColors.terraform },
  'terraform.tfvars': { type: 'terraform', color: iconColors.terraform },

  // Kubernetes / Helm
  'Chart.yaml': { type: 'helm', color: '#0f1689' },
  'values.yaml': { type: 'helm', color: '#0f1689' },
  'helmfile.yaml': { type: 'helm', color: '#0f1689' },

  // Elixir
  'mix.exs': { type: 'elixir', color: iconColors.elixir },
  'mix.lock': { type: 'elixir', color: iconColors.elixir },

  // Clojure
  'project.clj': { type: 'clojure', color: iconColors.clojure },
  'deps.edn': { type: 'clojure', color: iconColors.clojure },

  // Haskell
  'stack.yaml': { type: 'haskell', color: iconColors.haskell },
  'cabal.project': { type: 'haskell', color: iconColors.haskell },

  // Scala
  'build.sbt': { type: 'scala', color: iconColors.scala },

  // Nginx
  'nginx.conf': { type: 'nginx', color: iconColors.nginx },

  // HTACCESS
  '.htaccess': { type: 'apache', color: '#d22128' },

  // Editor config
  '.editorconfig': { type: 'editorconfig', color: '#e0efef' },

  // Browserslist
  '.browserslistrc': { type: 'browserslist', color: '#ffd539' },
  browserslist: { type: 'browserslist', color: '#ffd539' },

  // Node version
  '.nvmrc': { type: 'node', color: '#339933' },
  '.node-version': { type: 'node', color: '#339933' },

  // TypeScript
  'tsconfig.build.json': { type: 'tsconfig', color: iconColors.typescript },
  'tsconfig.app.json': { type: 'tsconfig', color: iconColors.typescript },
  'tsconfig.spec.json': { type: 'tsconfig', color: iconColors.typescript },

  // Angular
  'angular.json': { type: 'angular', color: '#dd0031' },
};

export function getFileIconInfo(filename: string): { type: string; color: string } {
  // Check special filenames first
  const lowerFilename = filename.toLowerCase();
  if (filenameMap[filename] || filenameMap[lowerFilename]) {
    return filenameMap[filename] || filenameMap[lowerFilename];
  }

  // Get extension
  const parts = filename.split('.');
  if (parts.length > 1) {
    const ext = parts.pop()!.toLowerCase();

    // Check for compound extensions like .d.ts
    if (parts.length > 1) {
      const compoundExt = `${parts.pop()}.${ext}`;
      if (extensionMap[compoundExt]) {
        return extensionMap[compoundExt];
      }
    }

    if (extensionMap[ext]) {
      return extensionMap[ext];
    }
  }

  // Default
  return { type: 'file', color: iconColors.text };
}

export function getFolderColor(isOpen: boolean): string {
  return isOpen ? iconColors.folderOpen : iconColors.folder;
}
