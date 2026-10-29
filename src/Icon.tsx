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
  | "shield";

export function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    build: "m15 12-8.373 8.373a1 1 0 1 1-3-3L12 9M18 15l4-4m-.5-3.5-1.914-1.914A2 2 0 0 1 19 8.172V7l-2.26-2.26a6 6 0 0 0-4.202-1.756L9 2.96l.92.82A6.18 6.18 0 0 1 12 8.4V10l2 2h1.172a2 2 0 0 1 1.414.586L18.5 14.5",
    run: "m7 4 14 8-14 8V4Z",
    debug: "m8 2 1.88 1.88M14.12 3.88 16 2M9 7.13v-1a3 3 0 1 1 6 0v1M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6Zm0 0v-9M6.53 9C4.6 8.8 3 7.1 3 5m3 8H2m1 8c0-2.1 1.7-3.9 3.8-4M20.97 5c0 2.1-1.6 3.8-3.5 4M22 13h-4m-.8 4c2.1.1 3.8 1.9 3.8 4",
    file: "M5 3h9l5 5v13H5V3Zm9 0v6h5M8 13h8m-8 4h6",
    project: "m3 7 9-4 9 4v11l-9 4-9-4V7Zm0 0 9 4 9-4m-9 4v11",
    save: "M4 3h13l4 4v14H3V3h1Zm3 0v7h10V3M7 21v-7h10v7",
    code: "m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18",
    rename: "m4 16 12-12 4 4L8 20H4v-4Zm10-10 4 4",
    problems: "M12 3 2 21h20L12 3Zm0 6v6m0 3v.5",
    terminal: "M4 5h16v14H4V5Zm4 5 3 3-3 3m6 0h4",
    tests: "M9 3h6v5l4 9a3 3 0 0 1-3 4H8a3 3 0 0 1-3-4l4-9V3Zm-5 9h16",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm6 13 4 4",
    settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4 0a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z",
    git: "M6 8v10M18 8v1c0 4-3 6-7 6H6M6 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4Zm12 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4Z",
    package: "m12 3 8 4v10l-8 4-8-4V7l8-4Zm-8 4 8 4 8-4m-8 4v10M8 9l8-4",
    stop: "M6 6h12v12H6V6Z",
    back: "m15 18-6-6 6-6",
    forward: "m9 18 6-6-6-6",
    chevron: "m6 9 6 6 6-6",
    shield: "M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6l-8-3Z",
    runconfig: "M4 7h6m4 0h6M4 17h10m4 0h2M12 4v6M18 14v6",
  };
  return <svg className={`icon icon-${name}`} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
