import { useRef } from "react";

export function Splitter({ label, value, min, max, horizontal = false, onChange }: {
  label: string; value: number; min: number; max: number; horizontal?: boolean; onChange: (value: number) => void;
}) {
  const drag = useRef<{ position: number; value: number } | null>(null);
  const update = (next: number) => onChange(Math.max(min, Math.min(max, next)));
  return <div className={`splitter ${horizontal ? "horizontal" : "vertical"}`} role="separator" tabIndex={0}
    aria-label={label} aria-orientation={horizontal ? "horizontal" : "vertical"} aria-valuenow={value} aria-valuemin={min} aria-valuemax={max}
    onPointerDown={(event) => {
      drag.current = { position: horizontal ? event.clientY : event.clientX, value };
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onPointerMove={(event) => {
      if (!drag.current) return;
      const delta = (horizontal ? event.clientY : event.clientX) - drag.current.position;
      update(drag.current.value + (horizontal ? -delta : delta));
    }}
    onPointerUp={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }}
    onKeyDown={(event) => {
      const direction = horizontal ? { ArrowUp: 16, ArrowDown: -16 } : { ArrowLeft: -16, ArrowRight: 16 };
      const delta = direction[event.key as keyof typeof direction];
      if (delta !== undefined) { event.preventDefault(); update(value + delta); }
      if (event.key === "Home" || event.key === "End") { event.preventDefault(); update(event.key === "Home" ? min : max); }
    }} />;
}
