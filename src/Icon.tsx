import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Blocks,
  Box,
  Bug,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronsDown,
  ChevronsUp,
  CodeXml,
  EllipsisVertical,
  Eye,
  File,
  FlaskConical,
  Folder,
  FolderTree,
  GitBranch,
  Hammer,
  LocateFixed,
  Minus,
  Network,
  Package,
  Pause,
  Pencil,
  Play,
  Plus,
  Redo2,
  RotateCcw,
  Save,
  Search,
  Settings,
  Shield,
  SlidersHorizontal,
  Square,
  X,
  SquareTerminal,
  StepForward,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";

export type IconName =
  | "build"
  | "run"
  | "debug"
  | "file"
  | "project"
  | "save"
  | "code"
  | "rename"
  | "problems"
  | "terminal"
  | "tests"
  | "search"
  | "settings"
  | "git"
  | "package"
  | "runconfig"
  | "stop"
  | "back"
  | "forward"
  | "chevron"
  | "plus"
  | "close"
  | "locate"
  | "expandAll"
  | "collapseAll"
  | "shield"
  | "resume"
  | "stepOver"
  | "stepInto"
  | "stepOut"
  | "pause"
  | "restart"
  | "folder"
  | "solution"
  | "solutionFolder"
  | "dependencies"
  | "eye"
  | "more"
  | "minimize";

const icons: Record<IconName, LucideIcon> = {
  build: Hammer,
  run: Play,
  debug: Bug,
  file: File,
  project: Box,
  save: Save,
  code: CodeXml,
  rename: Pencil,
  problems: TriangleAlert,
  terminal: SquareTerminal,
  tests: FlaskConical,
  search: Search,
  settings: Settings,
  git: GitBranch,
  package: Package,
  runconfig: SlidersHorizontal,
  stop: Square,
  back: ChevronLeft,
  forward: ChevronRight,
  chevron: ChevronDown,
  plus: Plus,
  close: X,
  locate: LocateFixed,
  expandAll: ChevronsDown,
  collapseAll: ChevronsUp,
  shield: Shield,
  resume: StepForward,
  stepOver: Redo2,
  stepInto: ArrowDownToLine,
  stepOut: ArrowUpFromLine,
  pause: Pause,
  restart: RotateCcw,
  folder: Folder,
  solution: Blocks,
  solutionFolder: FolderTree,
  dependencies: Network,
  eye: Eye,
  more: EllipsisVertical,
  minimize: Minus,
};

/** Lucide glyphs; stroke stays 1.5 screen px at any CSS size (`.icon *` uses non-scaling-stroke). */
export function Icon({ name }: { name: IconName }) {
  const Glyph = icons[name];
  const filled = name === "run" || name === "stop";
  return <Glyph className={`icon icon-${name}`} size={16} strokeWidth={1.5} fill={filled ? "currentColor" : "none"} aria-hidden="true" />;
}
