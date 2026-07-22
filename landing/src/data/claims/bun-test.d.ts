declare module 'bun:test' {
  interface Matchers {
    toBe(expected: unknown): void;
    toBeDefined(): void;
    toMatchObject(expected: object): void;
    toThrow(message?: string): void;
  }

  export function expect(actual: unknown): Matchers;
  export function test(name: string, body: () => void): void;
}
