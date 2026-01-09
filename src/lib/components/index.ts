// Layout components
export { default as AppLayout } from './layout/AppLayout.svelte';
export { default as Sidebar } from './layout/Sidebar.svelte';
export { default as MainContent } from './layout/MainContent.svelte';
export { default as StatusBar } from './layout/StatusBar.svelte';

// FileTree components
export { FileTree, FileTreeItem } from './filetree';
export type { FileEntry } from './filetree';

// Terminal components
export { Terminal } from './terminal';

// Editor components
export { Editor, getLanguageExtension, getLanguageName } from './editor';
