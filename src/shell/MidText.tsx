// Truncates in the middle so the distinctive suffix (extension, branch tail) stays readable; the full text is the parent's title.
export function MidText({ text }: { text: string }) {
  const tail = Math.min(12, Math.ceil(text.length * 0.35));
  if (text.length < 8) return <span className="mid">{text}</span>;
  return (
    <span className="mid">
      <span className="mid-head">{text.slice(0, -tail)}</span>
      <span className="mid-tail">{text.slice(-tail)}</span>
    </span>
  );
}
