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
