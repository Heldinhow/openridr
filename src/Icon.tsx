export function Icon({ name }: { name: "build" | "run" | "debug" | "file" | "project" | "save" | "code" | "rename" }) {
  const paths = {
    build: "m14 3 7 7-4 4-7-7 4-4ZM3 21l9-9m-6 6 2 2",
    run: "m7 4 14 8-14 8V4Z",
    debug: "M8 8h8v10H8V8Zm2-4h4v4M4 10h4m8 0h4M4 16h4m8 0h4M9 21l2-3m4 3-2-3",
    file: "M5 3h9l5 5v13H5V3Zm9 0v6h5M8 13h8m-8 4h6",
    project: "m3 7 9-4 9 4v11l-9 4-9-4V7Zm0 0 9 4 9-4m-9 4v11",
    save: "M4 3h13l4 4v14H3V3h1Zm3 0v7h10V3M7 21v-7h10v7",
    code: "m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18",
    rename: "m4 16 12-12 4 4L8 20H4v-4Zm10-10 4 4",
  };
  return <svg className={`icon icon-${name}`} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
