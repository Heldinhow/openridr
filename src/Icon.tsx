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
  | "stop";

export function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    build: "m14 3 7 7-4 4-7-7 4-4ZM3 21l9-9m-6 6 2 2",
    run: "m7 4 14 8-14 8V4Z",
    debug: "M8 8h8v10H8V8Zm2-4h4v4M4 10h4m8 0h4M4 16h4m8 0h4M9 21l2-3m4 3-2-3",
    file: "M5 3h9l5 5v13H5V3Zm9 0v6h5M8 13h8m-8 4h6",
    project: "m3 7 9-4 9 4v11l-9 4-9-4V7Zm0 0 9 4 9-4m-9 4v11",
    save: "M4 3h13l4 4v14H3V3h1Zm3 0v7h10V3M7 21v-7h10v7",
    code: "m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18",
    rename: "m4 16 12-12 4 4L8 20H4v-4Zm10-10 4 4",
    problems: "M12 3 2 21h20L12 3Zm0 6v6m0 3v.5",
    terminal: "M4 5h16v14H4V5Zm4 5 3 3-3 3m6 0h4",
    tests: "M9 3h6v5l4 9a3 3 0 0 1-3 4H8a3 3 0 0 1-3-4l4-9V3Zm-5 9h16",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm6 13 4 4",
    settings: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm0-5v3m0 12v3M3 12h3m12 0h3M5.6 5.6l2.1 2.1m8.6 8.6 2.1 2.1m0-12.8-2.1 2.1M7.7 16.3l-2.1 2.1",
    git: "M6 8v10M18 8v1c0 4-3 6-7 6H6M6 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4Zm12 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4Z",
    package: "m12 3 8 4v10l-8 4-8-4V7l8-4Zm-8 4 8 4 8-4m-8 4v10M8 9l8-4",
    stop: "M6 6h12v12H6V6Z",
    runconfig: "M4 7h6m4 0h6M4 17h10m4 0h2M12 4v6M18 14v6",
  };
  return <svg className={`icon icon-${name}`} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
