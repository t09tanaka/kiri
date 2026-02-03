export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_hidden: boolean;
  is_gitignored: boolean;
  /** Indicates the file is being copied (optimistic UI) */
  is_pending?: boolean;
}
