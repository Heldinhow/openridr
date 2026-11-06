// Minimal ambient types for `bun test`, so `tsc --noEmit` can check tests/unit without a
// new dependency. Only the matchers this folder uses are declared.
declare module "bun:test" {
  export function describe(name: string, body: () => void | Promise<void>): void;
  export function test(name: string, body: () => void | Promise<void>): void;
  export function expect(actual: unknown): {
    toBe(expected: unknown): void;
    toEqual(expected: unknown): void;
    toBeUndefined(): void;
    toBeNull(): void;
    toHaveLength(expected: number): void;
    not: {
      toBe(expected: unknown): void;
      toEqual(expected: unknown): void;
      toBeUndefined(): void;
      toBeNull(): void;
    };
  };
}
