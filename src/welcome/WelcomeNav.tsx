import { Icon } from "../Icon";

export type WelcomeView = "projects" | "toolchain";

type Props = { view: WelcomeView; onView: (view: WelcomeView) => void };

const items: { view: WelcomeView; label: string }[] = [
  { view: "projects", label: "Projects" },
  { view: "toolchain", label: "Toolchain" },
];

export function WelcomeNav({ view, onView }: Props) {
  return (
    <nav className="welcome-nav" aria-label="Welcome">
      <div className="welcome-logo">
        <span className="welcome-mark"><Icon name="code" /></span>
        <div><strong>OpenRidr</strong><span className="muted">C# workspace</span></div>
      </div>
      {items.map((item) => (
        <button
          key={item.view}
          type="button"
          className={`welcome-link${view === item.view ? " active" : ""}`}
          aria-current={view === item.view ? "page" : undefined}
          data-testid={`welcome-nav-${item.view}`}
          onClick={() => onView(item.view)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}
